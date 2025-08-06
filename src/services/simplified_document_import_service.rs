use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tempfile::NamedTempFile;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{TradocumentError, Result};

/// Simplified document import service without translation memory dependencies
pub struct SimplifiedDocumentImportService {
    supported_formats: Vec<String>,
}

/// Configuration for document import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    pub preserve_formatting: bool,
    pub extract_images: bool,
    pub chapter_mode: bool,
    pub target_language: String,
}

/// Result of importing a single document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentImportResult {
    pub success: bool,
    pub filename: String,
    pub title: String,
    pub content: String,
    pub language: String,
    pub chapter_number: Option<u32>,
    pub messages: Vec<String>,
    pub warnings: Vec<String>,
    pub processing_time_ms: u64,
}

/// Result of importing multiple documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiDocumentImportResult {
    pub success: bool,
    pub documents: Vec<DocumentImportResult>,
    pub total_files: usize,
    pub successful_imports: usize,
    pub errors: Vec<ImportError>,
    pub processing_time_ms: u64,
}

/// Import error for a specific file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub filename: String,
    pub error: String,
}

/// Chapter information extracted from documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: Uuid,
    pub chapter_number: u32,
    pub title: String,
    pub content: String,
    pub language: String,
    pub filename: String,
    pub created_at: DateTime<Utc>,
}

/// Progress information for import operations
#[derive(Debug, Clone)]
pub struct ImportProgress {
    pub current_file: String,
    pub progress_percent: u8,
    pub message: String,
}

/// Result of file validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileValidationResult {
    pub is_valid: bool,
    pub detected_format: Option<String>,
    pub file_size: u64,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl SimplifiedDocumentImportService {
    /// Create a new simplified document import service
    pub fn new() -> Self {
        Self {
            supported_formats: vec![
                "docx".to_string(),
                "doc".to_string(),
                "txt".to_string(),
                "md".to_string(),
            ],
        }
    }

    /// Get supported file formats
    pub fn supported_formats(&self) -> &[String] {
        &self.supported_formats
    }

    /// Check if a file format is supported
    pub fn is_format_supported(&self, filename: &str) -> bool {
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        
        self.supported_formats.contains(&extension)
    }

    /// Detect file format from file content (magic bytes)
    pub fn detect_format_from_content(&self, file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to open file: {}", e)))?;
        
        let mut buffer = [0; 8];
        file.read_exact(&mut buffer)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read file header: {}", e)))?;

        // Check magic bytes for different formats
        if buffer.starts_with(b"PK\x03\x04") {
            // ZIP-based format (likely DOCX)
            Ok("docx".to_string())
        } else if buffer.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
            // Microsoft Office legacy format (DOC)
            Ok("doc".to_string())
        } else {
            // Fall back to extension-based detection
            let extension = file_path.extension()
                .and_then(|ext| ext.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();
            
            if self.supported_formats.contains(&extension) {
                Ok(extension)
            } else {
                Err(TradocumentError::UnsupportedFormat(
                    format!("Cannot determine format for file: {}", file_path.display())
                ))
            }
        }
    }

    /// Validate file before import
    pub fn validate_file(&self, file_path: &Path) -> Result<FileValidationResult> {
        let mut result = FileValidationResult {
            is_valid: true,
            detected_format: None,
            file_size: 0,
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        // Check if file exists
        if !file_path.exists() {
            result.is_valid = false;
            result.errors.push(format!("File does not exist: {}", file_path.display()));
            return Ok(result);
        }

        // Get file size
        match std::fs::metadata(file_path) {
            Ok(metadata) => {
                result.file_size = metadata.len();
                
                // Warn about very large files
                if result.file_size > 50 * 1024 * 1024 { // 50MB
                    result.warnings.push("File is very large and may take time to process".to_string());
                }
                
                // Error on empty files
                if result.file_size == 0 {
                    result.is_valid = false;
                    result.errors.push("File is empty".to_string());
                    return Ok(result);
                }
            }
            Err(e) => {
                result.is_valid = false;
                result.errors.push(format!("Cannot read file metadata: {}", e));
                return Ok(result);
            }
        }

        // Detect and validate format
        match self.detect_format_from_content(file_path) {
            Ok(format) => {
                result.detected_format = Some(format.clone());
                
                // Format-specific validation
                match format.as_str() {
                    "docx" => {
                        if let Err(e) = self.validate_docx_file(file_path) {
                            result.warnings.push(format!("DOCX validation warning: {}", e));
                        }
                    }
                    "doc" => {
                        result.warnings.push("DOC format has limited support - consider converting to DOCX".to_string());
                    }
                    "txt" => {
                        // Check if file is actually text
                        if let Err(e) = self.validate_text_file(file_path) {
                            result.is_valid = false;
                            result.errors.push(format!("Text file validation failed: {}", e));
                        }
                    }
                    "md" => {
                        // Validate markdown syntax
                        if let Err(e) = self.validate_markdown_file(file_path) {
                            result.warnings.push(format!("Markdown validation warning: {}", e));
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                result.is_valid = false;
                result.errors.push(e.to_string());
            }
        }

        Ok(result)
    }

    /// Validate DOCX file structure
    fn validate_docx_file(&self, file_path: &Path) -> Result<()> {
        let mut file = File::open(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to open DOCX: {}", e)))?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read DOCX: {}", e)))?;

        // Try to parse as DOCX
        docx_rs::read_docx(&buffer)
            .map_err(|e| TradocumentError::Validation(format!("Invalid DOCX structure: {}", e)))?;

        Ok(())
    }

    /// Validate text file encoding
    fn validate_text_file(&self, file_path: &Path) -> Result<()> {
        let content = std::fs::read(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read text file: {}", e)))?;

        // Check if content is valid UTF-8
        match String::from_utf8(content) {
            Ok(_) => Ok(()),
            Err(_) => {
                // Try to detect if it's a different encoding
                Err(TradocumentError::Validation(
                    "File does not appear to be valid UTF-8 text".to_string()
                ))
            }
        }
    }

    /// Validate markdown file syntax
    fn validate_markdown_file(&self, file_path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read markdown file: {}", e)))?;

        // Basic markdown validation - check for common issues
        let lines: Vec<&str> = content.lines().collect();
        let mut issues = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            // Check for malformed links
            if line.contains('[') && line.contains(']') && !line.contains('(') {
                issues.push(format!("Line {}: Possible malformed link", line_num + 1));
            }
            
            // Check for unmatched code blocks
            if line.trim() == "```" {
                // This is a simple check - a full parser would be better
                let code_block_count = lines.iter().filter(|l| l.trim() == "```").count();
                if code_block_count % 2 != 0 {
                    issues.push("Unmatched code block delimiters".to_string());
                    break;
                }
            }
        }

        if !issues.is_empty() {
            return Err(TradocumentError::Validation(
                format!("Markdown validation issues: {}", issues.join(", "))
            ));
        }

        Ok(())
    }

    /// Import a single document file
    pub async fn import_document(
        &self,
        file_path: &Path,
        config: &ImportConfig,
    ) -> Result<DocumentImportResult> {
        let start_time = Instant::now();
        let mut messages = Vec::new();
        let mut warnings = Vec::new();

        let filename = file_path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        messages.push(format!("Starting import of {}", filename));

        // Validate file exists
        if !file_path.exists() {
            return Err(TradocumentError::FileError(
                format!("File does not exist: {}", file_path.display())
            ));
        }

        // Validate file format
        if !self.is_format_supported(&filename) {
            return Err(TradocumentError::UnsupportedFormat(
                format!("Unsupported file format: {}", filename)
            ));
        }

        // Convert document to markdown based on format
        let content = self.convert_to_markdown(file_path, config, &mut messages, &mut warnings).await?;
        
        // Extract title from filename
        let title = file_path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let processing_time = start_time.elapsed().as_millis() as u64;
        messages.push(format!("Import completed in {}ms", processing_time));

        Ok(DocumentImportResult {
            success: true,
            filename,
            title,
            content,
            language: config.target_language.clone(),
            chapter_number: None,
            messages,
            warnings,
            processing_time_ms: processing_time,
        })
    }

    /// Import multiple documents with chapter organization
    pub async fn import_multiple_documents(
        &self,
        file_paths: Vec<PathBuf>,
        config: &ImportConfig,
        progress_callback: Option<Box<dyn Fn(ImportProgress) + Send + Sync>>,
    ) -> Result<MultiDocumentImportResult> {
        let start_time = Instant::now();
        let total_files = file_paths.len();
        let mut documents = Vec::new();
        let mut errors = Vec::new();
        let mut successful_imports = 0;

        for (index, file_path) in file_paths.iter().enumerate() {
            let filename = file_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Report progress
            if let Some(ref callback) = progress_callback {
                let progress = ImportProgress {
                    current_file: filename.clone(),
                    progress_percent: ((index as f32 / total_files as f32) * 100.0) as u8,
                    message: format!("Processing {} ({}/{})", filename, index + 1, total_files),
                };
                callback(progress);
            }

            // Import individual document
            match self.import_document(file_path, config).await {
                Ok(mut result) => {
                    // Add chapter number if in chapter mode
                    if config.chapter_mode {
                        result.chapter_number = Some((index + 1) as u32);
                    }
                    documents.push(result);
                    successful_imports += 1;
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
        if let Some(ref callback) = progress_callback {
            let progress = ImportProgress {
                current_file: "Complete".to_string(),
                progress_percent: 100,
                message: format!("Imported {} of {} files", successful_imports, total_files),
            };
            callback(progress);
        }

        let processing_time = start_time.elapsed().as_millis() as u64;

        Ok(MultiDocumentImportResult {
            success: errors.is_empty(),
            documents,
            total_files,
            successful_imports,
            errors,
            processing_time_ms: processing_time,
        })
    }

    /// Convert document to markdown based on file format
    async fn convert_to_markdown(
        &self,
        file_path: &Path,
        config: &ImportConfig,
        messages: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        match extension.as_str() {
            "docx" => self.convert_docx_to_markdown(file_path, config, messages, warnings).await,
            "doc" => self.convert_doc_to_markdown(file_path, config, messages, warnings).await,
            "txt" => self.convert_txt_to_markdown(file_path, messages).await,
            "md" => self.load_markdown_file(file_path, messages).await,
            _ => Err(TradocumentError::UnsupportedFormat(
                format!("Unsupported format: {}", extension)
            )),
        }
    }

    /// Convert DOCX file to markdown with improved text extraction
    async fn convert_docx_to_markdown(
        &self,
        file_path: &Path,
        config: &ImportConfig,
        messages: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        messages.push("Converting DOCX to markdown".to_string());

        // Read the DOCX file
        let mut file = File::open(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to open DOCX file: {}", e)))?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read DOCX file: {}", e)))?;

        // Try using markdownify first (existing approach)
        match self.convert_with_markdownify(&buffer, file_path, warnings) {
            Ok(markdown) => {
                messages.push("Successfully converted using markdownify".to_string());
                Ok(self.post_process_markdown(markdown, config, warnings))
            }
            Err(_) => {
                warnings.push("Markdownify conversion failed, trying docx-rs".to_string());
                // Fallback to docx-rs approach
                self.convert_with_docx_rs(&buffer, file_path, config, messages, warnings)
            }
        }
    }

    /// Convert DOCX using markdownify library
    fn convert_with_markdownify(
        &self,
        docx_bytes: &[u8],
        file_path: &Path,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        // Create a temporary file for markdownify
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| TradocumentError::FileError(format!("Failed to create temp file: {}", e)))?;
        
        temp_file.write_all(docx_bytes)
            .map_err(|e| TradocumentError::FileError(format!("Failed to write to temp file: {}", e)))?;
        
        let temp_path = temp_file.path();
        
        match markdownify::convert(temp_path, None) {
            Ok(markdown) => {
                if markdown.trim().is_empty() {
                    let filename = file_path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("document");
                    warnings.push(format!("Document {} appears to be empty", filename));
                    Ok(format!("# {}\n\n*No content could be extracted from the document.*", filename))
                } else {
                    Ok(markdown)
                }
            }
            Err(e) => Err(TradocumentError::DocumentImport(
                format!("Markdownify conversion failed: {}", e)
            )),
        }
    }

    /// Convert DOCX using docx-rs library for better text extraction
    fn convert_with_docx_rs(
        &self,
        docx_bytes: &[u8],
        file_path: &Path,
        config: &ImportConfig,
        messages: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        messages.push("Using docx-rs for text extraction".to_string());

        // Parse DOCX document
        let docx = docx_rs::read_docx(docx_bytes)
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to parse DOCX: {}", e)))?;

        let filename = file_path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Untitled");

        let mut markdown = String::new();
        
        // Add document title
        markdown.push_str(&format!("# {}\n\n", filename));

        // Extract text content from the document
        let extracted_text = self.extract_text_from_docx_document(&docx, config, warnings)?;
        
        if extracted_text.trim().is_empty() {
            warnings.push("No text content could be extracted from DOCX".to_string());
            markdown.push_str("*No content could be extracted from the document.*\n\n");
        } else {
            // Process extracted text into markdown
            markdown.push_str(&self.process_extracted_text(&extracted_text, config, warnings));
        }

        messages.push("DOCX text extraction completed".to_string());
        Ok(markdown)
    }

    /// Extract text from DOCX document structure
    fn extract_text_from_docx_document(
        &self,
        docx: &docx_rs::Docx,
        config: &ImportConfig,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        let mut text_content = String::new();
        
        // Extract text from document body
        for child in &docx.document.body.children {
            match child {
                docx_rs::DocumentChild::Paragraph(paragraph) => {
                    let paragraph_text = self.extract_paragraph_text(paragraph, config);
                    if !paragraph_text.trim().is_empty() {
                        text_content.push_str(&paragraph_text);
                        text_content.push('\n');
                    }
                }
                docx_rs::DocumentChild::Table(table) => {
                    if config.preserve_formatting {
                        let table_text = self.extract_table_text(table);
                        if !table_text.trim().is_empty() {
                            text_content.push_str(&table_text);
                            text_content.push('\n');
                        }
                        warnings.push("Table found - formatting may need manual review".to_string());
                    }
                }
                _ => {
                    // Handle other document elements as needed
                }
            }
        }

        Ok(text_content)
    }

    /// Extract text from a paragraph
    fn extract_paragraph_text(&self, paragraph: &docx_rs::Paragraph, config: &ImportConfig) -> String {
        let mut paragraph_text = String::new();
        let mut is_heading = false;
        
        // Check if this paragraph is a heading based on style
        if let Some(ref properties) = paragraph.property {
            if let Some(ref style) = properties.style {
                if style.val.to_lowercase().contains("heading") {
                    is_heading = true;
                }
            }
        }

        // Extract text from runs
        for child in &paragraph.children {
            match child {
                docx_rs::ParagraphChild::Run(run) => {
                    for run_child in &run.children {
                        match run_child {
                            docx_rs::RunChild::Text(text) => {
                                paragraph_text.push_str(&text.text);
                            }
                            docx_rs::RunChild::Tab(_) => {
                                paragraph_text.push(' ');
                            }
                            docx_rs::RunChild::Break(_) => {
                                paragraph_text.push('\n');
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        // Format as heading if detected
        if is_heading && config.preserve_formatting && !paragraph_text.trim().is_empty() {
            format!("## {}\n", paragraph_text.trim())
        } else {
            paragraph_text
        }
    }

    /// Extract text from a table
    fn extract_table_text(&self, table: &docx_rs::Table) -> String {
        let mut table_text = String::new();
        
        for row in &table.rows {
            let mut row_cells = Vec::new();
            
            for cell in &row.cells {
                let mut cell_text = String::new();
                
                for paragraph in &cell.children {
                    if let docx_rs::TableCellChild::Paragraph(para) = paragraph {
                        let para_text = self.extract_paragraph_text(para, &ImportConfig {
                            preserve_formatting: false,
                            extract_images: false,
                            chapter_mode: false,
                            target_language: "en".to_string(),
                        });
                        cell_text.push_str(&para_text.trim());
                        cell_text.push(' ');
                    }
                }
                
                row_cells.push(cell_text.trim().to_string());
            }
            
            if !row_cells.is_empty() {
                table_text.push_str(&format!("| {} |\n", row_cells.join(" | ")));
            }
        }
        
        if !table_text.is_empty() {
            // Add table header separator for the first row
            let lines: Vec<&str> = table_text.lines().collect();
            if lines.len() > 0 {
                let separator_count = lines[0].matches('|').count() - 1;
                let separator = format!("| {} |\n", vec!["---"; separator_count].join(" | "));
                table_text = format!("{}{}{}", lines[0], "\n", separator);
                for line in &lines[1..] {
                    table_text.push_str(line);
                    table_text.push('\n');
                }
            }
        }
        
        table_text
    }

    /// Process extracted text into proper markdown format
    fn process_extracted_text(&self, text: &str, config: &ImportConfig, warnings: &mut Vec<String>) -> String {
        let mut processed = String::new();
        let lines: Vec<&str> = text.lines().collect();
        
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                processed.push('\n');
                continue;
            }

            // Check if line is already a heading
            if trimmed.starts_with('#') {
                processed.push_str(trimmed);
                processed.push('\n');
            }
            // Check if line looks like a heading (all caps, short, etc.)
            else if config.preserve_formatting && self.looks_like_heading(trimmed) {
                processed.push_str(&format!("## {}\n", trimmed));
            }
            // Regular paragraph
            else {
                processed.push_str(trimmed);
                processed.push_str("\n\n");
            }
        }

        processed
    }

    /// Heuristic to detect if a line looks like a heading
    fn looks_like_heading(&self, line: &str) -> bool {
        let line = line.trim();
        
        // Skip very long lines
        if line.len() > 100 {
            return false;
        }
        
        // Check if mostly uppercase
        let uppercase_count = line.chars().filter(|c| c.is_uppercase()).count();
        let letter_count = line.chars().filter(|c| c.is_alphabetic()).count();
        
        if letter_count > 0 && (uppercase_count as f32 / letter_count as f32) > 0.7 {
            return true;
        }
        
        // Check if it ends with a colon (common heading pattern)
        if line.ends_with(':') && line.len() < 50 {
            return true;
        }
        
        false
    }

    /// Post-process markdown to improve formatting
    fn post_process_markdown(&self, markdown: String, config: &ImportConfig, warnings: &mut Vec<String>) -> String {
        let mut processed = markdown;
        
        // Clean up excessive whitespace
        processed = processed
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        
        // Normalize heading levels if formatting is preserved
        if config.preserve_formatting {
            processed = self.normalize_heading_levels(processed, warnings);
        }
        
        // Improve list formatting
        processed = self.improve_list_formatting(processed);
        
        // Clean up excessive blank lines
        processed = self.clean_excessive_blank_lines(processed);
        
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

    /// Improve list formatting
    fn improve_list_formatting(&self, content: String) -> String {
        content
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+') {
                    // Ensure consistent list marker
                    let content = trimmed[1..].trim();
                    format!("- {}", content)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Clean up excessive blank lines
    fn clean_excessive_blank_lines(&self, content: String) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut processed_lines = Vec::new();
        let mut blank_line_count = 0;
        
        for line in lines {
            if line.trim().is_empty() {
                blank_line_count += 1;
                if blank_line_count <= 2 {
                    processed_lines.push(line.to_string());
                }
            } else {
                blank_line_count = 0;
                processed_lines.push(line.to_string());
            }
        }
        
        processed_lines.join("\n")
    }
}

impl Default for SimplifiedDocumentImportService {
    fn default() -> Self {
        Self::new()
    }
}impl Simpl
ifiedDocumentImportService {
    /// Convert DOC file to markdown (enhanced text extraction)
    async fn convert_doc_to_markdown(
        &self,
        file_path: &Path,
        config: &ImportConfig,
        messages: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        messages.push("Converting DOC file (legacy format)".to_string());
        warnings.push("DOC format support is limited - consider converting to DOCX".to_string());

        let filename = file_path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Untitled");

        let mut markdown = format!("# {}\n\n", filename);
        
        // Try enhanced text extraction first
        match self.extract_doc_text_enhanced(file_path, messages, warnings) {
            Ok(extracted_text) => {
                if !extracted_text.trim().is_empty() {
                    markdown.push_str(&self.process_extracted_text(&extracted_text, config, warnings));
                    messages.push("Successfully extracted text from DOC file".to_string());
                } else {
                    // Fall back to basic extraction
                    markdown.push_str(&self.create_doc_fallback_content(file_path, warnings)?);
                }
            }
            Err(_) => {
                // Fall back to basic extraction
                markdown.push_str(&self.create_doc_fallback_content(file_path, warnings)?);
            }
        }

        Ok(markdown)
    }

    /// Enhanced DOC text extraction using multiple strategies
    fn extract_doc_text_enhanced(
        &self,
        file_path: &Path,
        messages: &mut Vec<String>,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        let bytes = std::fs::read(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read DOC file: {}", e)))?;

        // Strategy 1: Look for text in the document stream
        if let Ok(text) = self.extract_doc_text_from_stream(&bytes) {
            if !text.trim().is_empty() {
                messages.push("Extracted text using stream parsing".to_string());
                return Ok(text);
            }
        }

        // Strategy 2: Extract readable text from binary data
        let binary_text = String::from_utf8_lossy(&bytes);
        let readable_text = self.extract_readable_text_from_binary_enhanced(&binary_text);
        
        if !readable_text.trim().is_empty() {
            messages.push("Extracted text using binary parsing".to_string());
            return Ok(readable_text);
        }

        warnings.push("No readable text could be extracted from DOC file".to_string());
        Err(TradocumentError::DocumentImport("No text extracted".to_string()))
    }

    /// Extract text from DOC file stream (simplified approach)
    fn extract_doc_text_from_stream(&self, bytes: &[u8]) -> Result<String> {
        // Look for text patterns in the DOC file structure
        // This is a simplified approach - full DOC parsing is very complex
        
        let mut text_content = String::new();
        let mut i = 0;
        
        while i < bytes.len() {
            // Look for text sequences (sequences of printable ASCII characters)
            if bytes[i].is_ascii_graphic() || bytes[i] == b' ' {
                let mut word = String::new();
                let start = i;
                
                // Collect consecutive printable characters
                while i < bytes.len() && (bytes[i].is_ascii_graphic() || bytes[i] == b' ') {
                    word.push(bytes[i] as char);
                    i += 1;
                }
                
                // Only include words that look like real text
                if word.len() > 3 && self.looks_like_real_text(&word) {
                    text_content.push_str(&word);
                    text_content.push(' ');
                }
            } else {
                i += 1;
            }
        }
        
        if text_content.trim().is_empty() {
            return Err(TradocumentError::DocumentImport("No text found".to_string()));
        }
        
        Ok(self.clean_extracted_doc_text(text_content))
    }

    /// Check if extracted text looks like real content
    fn looks_like_real_text(&self, text: &str) -> bool {
        let text = text.trim();
        
        // Skip very short or very long sequences
        if text.len() < 3 || text.len() > 200 {
            return false;
        }
        
        // Must contain at least some letters
        let letter_count = text.chars().filter(|c| c.is_alphabetic()).count();
        if letter_count < 2 {
            return false;
        }
        
        // Skip sequences that are mostly numbers or special characters
        let special_count = text.chars().filter(|c| !c.is_alphanumeric() && !c.is_whitespace()).count();
        if special_count > text.len() / 2 {
            return false;
        }
        
        // Skip common binary patterns
        if text.contains("Microsoft") && text.contains("Word") && text.len() < 50 {
            return false;
        }
        
        true
    }

    /// Enhanced binary text extraction with better filtering
    fn extract_readable_text_from_binary_enhanced(&self, binary_text: &str) -> String {
        let mut readable_text = String::new();
        let mut current_word = String::new();
        let mut words = Vec::new();
        
        for ch in binary_text.chars() {
            if ch.is_ascii_graphic() || ch.is_whitespace() {
                if ch.is_whitespace() {
                    if !current_word.is_empty() && current_word.len() > 2 {
                        words.push(current_word.clone());
                    }
                    current_word.clear();
                } else {
                    current_word.push(ch);
                }
            } else {
                // Non-printable character, end current word
                if !current_word.is_empty() && current_word.len() > 2 {
                    words.push(current_word.clone());
                }
                current_word.clear();
            }
        }
        
        // Add final word
        if !current_word.is_empty() && current_word.len() > 2 {
            words.push(current_word);
        }
        
        // Filter and clean words
        let filtered_words: Vec<String> = words
            .into_iter()
            .filter(|word| self.looks_like_real_text(word))
            .map(|word| word.trim().to_string())
            .filter(|word| !word.is_empty())
            .collect();
        
        // Group words into sentences/paragraphs
        let mut current_sentence = String::new();
        for word in filtered_words {
            current_sentence.push_str(&word);
            current_sentence.push(' ');
            
            // End sentence on punctuation or after reasonable length
            if word.ends_with('.') || word.ends_with('!') || word.ends_with('?') || current_sentence.len() > 100 {
                readable_text.push_str(current_sentence.trim());
                readable_text.push_str("\n\n");
                current_sentence.clear();
            }
        }
        
        // Add remaining sentence
        if !current_sentence.trim().is_empty() {
            readable_text.push_str(current_sentence.trim());
        }
        
        readable_text
    }

    /// Clean extracted DOC text
    fn clean_extracted_doc_text(&self, text: String) -> String {
        text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && line.len() > 3)
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Create fallback content for DOC files when extraction fails
    fn create_doc_fallback_content(&self, file_path: &Path, warnings: &mut Vec<String>) -> Result<String> {
        warnings.push("Using fallback content for DOC file - manual conversion recommended".to_string());
        
        let filename = file_path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Untitled");

        let mut content = String::new();
        content.push_str("## Legacy DOC Import\n\n");
        content.push_str("This document was imported from a legacy DOC file.\n\n");
        content.push_str("**Note**: Full DOC support requires additional conversion tools.\n");
        content.push_str("Please convert to DOCX format for better text extraction.\n\n");
        content.push_str("### Recommended Actions\n\n");
        content.push_str("1. Open the original DOC file in Microsoft Word\n");
        content.push_str("2. Save as DOCX format\n");
        content.push_str("3. Re-import the DOCX file\n\n");
        content.push_str(&format!("**Original file**: {}\n", filename));
        
        Ok(content)
    }

    /// Extract readable text from binary data (basic approach for DOC files)
    fn extract_readable_text_from_binary(&self, binary_text: &str) -> String {
        let mut readable_text = String::new();
        let mut current_word = String::new();
        
        for ch in binary_text.chars() {
            if ch.is_ascii_graphic() || ch.is_whitespace() {
                if ch.is_whitespace() {
                    if !current_word.is_empty() && current_word.len() > 2 {
                        readable_text.push_str(&current_word);
                        readable_text.push(' ');
                    }
                    current_word.clear();
                } else {
                    current_word.push(ch);
                }
            } else {
                // Non-printable character, end current word
                if !current_word.is_empty() && current_word.len() > 2 {
                    readable_text.push_str(&current_word);
                    readable_text.push(' ');
                }
                current_word.clear();
            }
        }
        
        // Add final word
        if !current_word.is_empty() && current_word.len() > 2 {
            readable_text.push_str(&current_word);
        }
        
        // Clean up the extracted text
        readable_text
            .split_whitespace()
            .filter(|word| word.len() > 2 && word.chars().any(|c| c.is_alphabetic()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Convert text file to markdown with enhanced formatting detection
    async fn convert_txt_to_markdown(
        &self,
        file_path: &Path,
        messages: &mut Vec<String>,
    ) -> Result<String> {
        messages.push("Converting text file to markdown".to_string());

        let content = std::fs::read_to_string(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read text file: {}", e)))?;

        let filename = file_path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Untitled");

        let mut markdown = format!("# {}\n\n", filename);
        
        // Enhanced text processing with better structure detection
        markdown.push_str(&self.convert_plain_text_to_markdown(&content));

        messages.push("Text file converted successfully".to_string());
        Ok(markdown)
    }

    /// Convert plain text to markdown with structure detection
    fn convert_plain_text_to_markdown(&self, content: &str) -> String {
        let mut markdown = String::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i].trim();
            
            if line.is_empty() {
                // Skip empty lines but preserve paragraph breaks
                markdown.push('\n');
                i += 1;
                continue;
            }
            
            // Check for different content types
            if self.looks_like_heading(line) {
                // Heading
                markdown.push_str(&format!("## {}\n\n", line));
            } else if self.looks_like_list_item(line) {
                // List item - collect consecutive list items
                let list_items = self.collect_list_items(&lines, &mut i);
                for item in list_items {
                    markdown.push_str(&format!("- {}\n", item));
                }
                markdown.push('\n');
                continue; // i is already advanced by collect_list_items
            } else if self.looks_like_numbered_list(line) {
                // Numbered list
                let list_items = self.collect_numbered_list_items(&lines, &mut i);
                for (num, item) in list_items.iter().enumerate() {
                    markdown.push_str(&format!("{}. {}\n", num + 1, item));
                }
                markdown.push('\n');
                continue; // i is already advanced
            } else {
                // Regular paragraph - collect until empty line or different content type
                let paragraph = self.collect_paragraph(&lines, &mut i);
                markdown.push_str(&paragraph);
                markdown.push_str("\n\n");
                continue; // i is already advanced
            }
            
            i += 1;
        }
        
        markdown
    }

    /// Check if line looks like a list item
    fn looks_like_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("- ") || 
        trimmed.starts_with("* ") || 
        trimmed.starts_with("+ ") ||
        trimmed.starts_with("• ")
    }

    /// Check if line looks like a numbered list item
    fn looks_like_numbered_list(&self, line: &str) -> bool {
        let trimmed = line.trim();
        if let Some(dot_pos) = trimmed.find('.') {
            if dot_pos < 4 { // Reasonable number length
                let number_part = &trimmed[..dot_pos];
                number_part.chars().all(|c| c.is_ascii_digit()) && 
                trimmed.len() > dot_pos + 2 && // Has content after dot
                trimmed.chars().nth(dot_pos + 1) == Some(' ')
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Collect consecutive list items
    fn collect_list_items(&self, lines: &[&str], i: &mut usize) -> Vec<String> {
        let mut items = Vec::new();
        
        while *i < lines.len() {
            let line = lines[*i].trim();
            
            if line.is_empty() {
                break;
            }
            
            if self.looks_like_list_item(line) {
                // Extract item text (remove bullet)
                let item_text = if line.starts_with("- ") {
                    &line[2..]
                } else if line.starts_with("* ") || line.starts_with("+ ") {
                    &line[2..]
                } else if line.starts_with("• ") {
                    &line[2..]
                } else {
                    line
                };
                items.push(item_text.trim().to_string());
            } else {
                break;
            }
            
            *i += 1;
        }
        
        items
    }

    /// Collect consecutive numbered list items
    fn collect_numbered_list_items(&self, lines: &[&str], i: &mut usize) -> Vec<String> {
        let mut items = Vec::new();
        
        while *i < lines.len() {
            let line = lines[*i].trim();
            
            if line.is_empty() {
                break;
            }
            
            if self.looks_like_numbered_list(line) {
                // Extract item text (remove number and dot)
                if let Some(dot_pos) = line.find('.') {
                    let item_text = &line[dot_pos + 1..].trim();
                    items.push(item_text.to_string());
                }
            } else {
                break;
            }
            
            *i += 1;
        }
        
        items
    }

    /// Collect paragraph text until empty line or different content type
    fn collect_paragraph(&self, lines: &[&str], i: &mut usize) -> String {
        let mut paragraph = String::new();
        
        while *i < lines.len() {
            let line = lines[*i].trim();
            
            if line.is_empty() {
                break;
            }
            
            // Stop if we encounter a different content type
            if self.looks_like_heading(line) || 
               self.looks_like_list_item(line) || 
               self.looks_like_numbered_list(line) {
                break;
            }
            
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(line);
            
            *i += 1;
        }
        
        paragraph
    }

    /// Load markdown file directly
    async fn load_markdown_file(
        &self,
        file_path: &Path,
        messages: &mut Vec<String>,
    ) -> Result<String> {
        messages.push("Loading markdown file".to_string());

        let content = std::fs::read_to_string(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read markdown file: {}", e)))?;

        messages.push("Markdown file loaded successfully".to_string());
        Ok(content)
    }

    /// Create chapters from multiple imported documents with enhanced organization
    pub fn create_chapters_from_documents(
        &self,
        import_results: &[DocumentImportResult],
        project_id: Option<Uuid>,
        organization_config: &ChapterOrganizationConfig,
    ) -> Result<Vec<Chapter>> {
        let mut chapters = Vec::new();
        
        // Sort documents based on organization strategy
        let sorted_results = self.sort_documents_for_chapters(import_results, organization_config)?;
        
        for (index, result) in sorted_results.iter().enumerate() {
            let chapter_number = if organization_config.preserve_original_numbering {
                result.chapter_number.unwrap_or((index + 1) as u32)
            } else {
                (index + 1) as u32
            };
            
            let title = if organization_config.auto_generate_titles {
                self.generate_chapter_title(result, chapter_number, organization_config)
            } else {
                result.title.clone()
            };
            
            let chapter = Chapter {
                id: Uuid::new_v4(),
                chapter_number,
                title,
                content: self.format_chapter_content(result, organization_config),
                language: result.language.clone(),
                filename: result.filename.clone(),
                created_at: Utc::now(),
            };
            chapters.push(chapter);
        }
        
        Ok(chapters)
    }

    /// Sort documents for chapter organization
    fn sort_documents_for_chapters(
        &self,
        import_results: &[DocumentImportResult],
        config: &ChapterOrganizationConfig,
    ) -> Result<Vec<DocumentImportResult>> {
        let mut sorted_results = import_results.to_vec();
        
        match config.sorting_strategy {
            ChapterSortingStrategy::Filename => {
                sorted_results.sort_by(|a, b| a.filename.cmp(&b.filename));
            }
            ChapterSortingStrategy::Title => {
                sorted_results.sort_by(|a, b| a.title.cmp(&b.title));
            }
            ChapterSortingStrategy::FileSize => {
                sorted_results.sort_by(|a, b| a.content.len().cmp(&b.content.len()));
            }
            ChapterSortingStrategy::Language => {
                sorted_results.sort_by(|a, b| a.language.cmp(&b.language));
            }
            ChapterSortingStrategy::Custom(ref custom_order) => {
                // Sort based on custom filename order
                sorted_results.sort_by(|a, b| {
                    let a_index = custom_order.iter().position(|f| f == &a.filename).unwrap_or(usize::MAX);
                    let b_index = custom_order.iter().position(|f| f == &b.filename).unwrap_or(usize::MAX);
                    a_index.cmp(&b_index)
                });
            }
            ChapterSortingStrategy::ImportOrder => {
                // Keep original order
            }
        }
        
        Ok(sorted_results)
    }

    /// Generate chapter title based on configuration
    fn generate_chapter_title(
        &self,
        result: &DocumentImportResult,
        chapter_number: u32,
        config: &ChapterOrganizationConfig,
    ) -> String {
        match config.title_generation_strategy {
            ChapterTitleStrategy::FromFilename => {
                let base_name = Path::new(&result.filename)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or(&result.filename);
                
                if config.include_chapter_numbers {
                    format!("Chapter {}: {}", chapter_number, self.clean_title(base_name))
                } else {
                    self.clean_title(base_name)
                }
            }
            ChapterTitleStrategy::FromContent => {
                // Extract title from content
                if let Some(extracted_title) = self.extract_title_from_content(&result.content) {
                    if config.include_chapter_numbers {
                        format!("Chapter {}: {}", chapter_number, extracted_title)
                    } else {
                        extracted_title
                    }
                } else {
                    // Fall back to filename
                    let base_name = Path::new(&result.filename)
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or(&result.filename);
                    
                    if config.include_chapter_numbers {
                        format!("Chapter {}: {}", chapter_number, self.clean_title(base_name))
                    } else {
                        self.clean_title(base_name)
                    }
                }
            }
            ChapterTitleStrategy::NumberedOnly => {
                format!("Chapter {}", chapter_number)
            }
            ChapterTitleStrategy::Custom(ref template) => {
                template
                    .replace("{number}", &chapter_number.to_string())
                    .replace("{filename}", &result.filename)
                    .replace("{title}", &result.title)
                    .replace("{language}", &result.language)
            }
        }
    }

    /// Clean title text for better formatting
    fn clean_title(&self, title: &str) -> String {
        title
            .replace('_', " ")
            .replace('-', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Format chapter content based on configuration
    fn format_chapter_content(&self, result: &DocumentImportResult, config: &ChapterOrganizationConfig) -> String {
        let mut content = result.content.clone();
        
        if config.add_chapter_metadata {
            let metadata = self.create_chapter_metadata(result);
            content = format!("{}\n\n{}", metadata, content);
        }
        
        if config.add_navigation_links {
            // Add navigation placeholder (would be filled in by the UI)
            content = format!("{}\n\n---\n*Navigation: [Previous] | [Next] | [Contents]*\n", content);
        }
        
        content
    }

    /// Create chapter metadata section
    fn create_chapter_metadata(&self, result: &DocumentImportResult) -> String {
        let mut metadata = String::new();
        metadata.push_str("<!-- Chapter Metadata -->\n");
        metadata.push_str(&format!("<!-- Source File: {} -->\n", result.filename));
        metadata.push_str(&format!("<!-- Language: {} -->\n", result.language));
        metadata.push_str(&format!("<!-- Import Date: {} -->\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        if !result.warnings.is_empty() {
            metadata.push_str("<!-- Import Warnings:\n");
            for warning in &result.warnings {
                metadata.push_str(&format!("  - {}\n", warning));
            }
            metadata.push_str("-->\n");
        }
        
        metadata.push_str("<!-- End Metadata -->\n");
        metadata
    }

    /// Organize chapters by language for multilingual projects
    pub fn organize_chapters_by_language(
        &self,
        chapters: &[Chapter],
    ) -> HashMap<String, Vec<Chapter>> {
        let mut language_chapters: HashMap<String, Vec<Chapter>> = HashMap::new();
        
        for chapter in chapters {
            language_chapters
                .entry(chapter.language.clone())
                .or_default()
                .push(chapter.clone());
        }
        
        // Sort chapters within each language by chapter number
        for chapters_list in language_chapters.values_mut() {
            chapters_list.sort_by_key(|c| c.chapter_number);
        }
        
        language_chapters
    }

    /// Create table of contents from chapters
    pub fn create_table_of_contents(&self, chapters: &[Chapter], config: &TocConfig) -> String {
        let mut toc = String::new();
        
        toc.push_str(&format!("# {}\n\n", config.title));
        
        if config.group_by_language {
            let language_chapters = self.organize_chapters_by_language(chapters);
            
            for (language, lang_chapters) in language_chapters {
                toc.push_str(&format!("## {} ({})\n\n", 
                    self.get_language_display_name(&language), 
                    language.to_uppercase()
                ));
                
                for chapter in lang_chapters {
                    let link = if config.include_links {
                        format!("[{}](#{}-{})", 
                            chapter.title, 
                            chapter.chapter_number,
                            chapter.title.to_lowercase().replace(' ', "-")
                        )
                    } else {
                        chapter.title.clone()
                    };
                    
                    toc.push_str(&format!("{}. {}\n", chapter.chapter_number, link));
                }
                toc.push('\n');
            }
        } else {
            let mut sorted_chapters = chapters.to_vec();
            sorted_chapters.sort_by_key(|c| c.chapter_number);
            
            for chapter in sorted_chapters {
                let link = if config.include_links {
                    format!("[{}](#{}-{})", 
                        chapter.title, 
                        chapter.chapter_number,
                        chapter.title.to_lowercase().replace(' ', "-")
                    )
                } else {
                    chapter.title.clone()
                };
                
                let language_indicator = if config.show_language_indicators {
                    format!(" ({})", chapter.language.to_uppercase())
                } else {
                    String::new()
                };
                
                toc.push_str(&format!("{}. {}{}\n", chapter.chapter_number, link, language_indicator));
            }
        }
        
        toc
    }

    /// Get display name for language code
    fn get_language_display_name(&self, language_code: &str) -> &str {
        match language_code {
            "en" => "English",
            "es" => "Spanish",
            "fr" => "French",
            "de" => "German",
            "it" => "Italian",
            "nl" => "Dutch",
            "pt" => "Portuguese",
            "ru" => "Russian",
            "ja" => "Japanese",
            "zh" => "Chinese",
            _ => language_code,
        }
    }

    /// Validate chapter organization
    pub fn validate_chapter_organization(&self, chapters: &[Chapter]) -> ChapterValidationResult {
        let mut result = ChapterValidationResult {
            is_valid: true,
            warnings: Vec::new(),
            errors: Vec::new(),
            statistics: ChapterStatistics::default(),
        };
        
        // Check for duplicate chapter numbers
        let mut chapter_numbers: HashMap<u32, Vec<String>> = HashMap::new();
        for chapter in chapters {
            chapter_numbers
                .entry(chapter.chapter_number)
                .or_default()
                .push(chapter.title.clone());
        }
        
        for (number, titles) in chapter_numbers {
            if titles.len() > 1 {
                result.warnings.push(format!(
                    "Chapter number {} is used by multiple chapters: {}", 
                    number, 
                    titles.join(", ")
                ));
            }
        }
        
        // Check for missing chapter numbers in sequence
        let mut numbers: Vec<u32> = chapters.iter().map(|c| c.chapter_number).collect();
        numbers.sort();
        
        for i in 1..numbers.len() {
            if numbers[i] != numbers[i-1] + 1 && numbers[i] != numbers[i-1] {
                result.warnings.push(format!(
                    "Chapter numbering gap detected: {} followed by {}", 
                    numbers[i-1], 
                    numbers[i]
                ));
            }
        }
        
        // Calculate statistics
        result.statistics.total_chapters = chapters.len();
        result.statistics.languages = chapters.iter()
            .map(|c| c.language.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        result.statistics.total_content_length = chapters.iter()
            .map(|c| c.content.len())
            .sum();
        result.statistics.average_chapter_length = if chapters.is_empty() {
            0
        } else {
            result.statistics.total_content_length / chapters.len()
        };
        
        result
    }

    /// Detect language from filename patterns
    pub fn detect_language_from_filename(&self, filename: &str) -> Option<String> {
        let filename_lower = filename.to_lowercase();

        // Check for common language indicators in filename
        if filename_lower.contains("_en") || filename_lower.contains("-en") || filename_lower.contains("english") {
            Some("en".to_string())
        } else if filename_lower.contains("_es") || filename_lower.contains("-es") || filename_lower.contains("spanish") {
            Some("es".to_string())
        } else if filename_lower.contains("_fr") || filename_lower.contains("-fr") || filename_lower.contains("french") {
            Some("fr".to_string())
        } else if filename_lower.contains("_de") || filename_lower.contains("-de") || filename_lower.contains("german") {
            Some("de".to_string())
        } else if filename_lower.contains("_it") || filename_lower.contains("-it") || filename_lower.contains("italian") {
            Some("it".to_string())
        } else if filename_lower.contains("_nl") || filename_lower.contains("-nl") || filename_lower.contains("dutch") {
            Some("nl".to_string())
        } else {
            None
        }
    }

    /// Extract title from document content
    pub fn extract_title_from_content(&self, content: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        
        for line in lines.iter().take(10) { // Check first 10 lines
            let trimmed = line.trim();
            if trimmed.starts_with("# ") {
                return Some(trimmed[2..].trim().to_string());
            }
        }
        
        None
    }

    /// Validate import configuration
    pub fn validate_import_config(&self, config: &ImportConfig) -> Result<()> {
        if config.target_language.is_empty() {
            return Err(TradocumentError::Validation(
                "Target language cannot be empty".to_string()
            ));
        }

        // Add more validation as needed
        Ok(())
    }

    /// Get import statistics
    pub fn get_import_statistics(&self, results: &[DocumentImportResult]) -> ImportStatistics {
        let total_files = results.len();
        let successful_imports = results.iter().filter(|r| r.success).count();
        let total_warnings = results.iter().map(|r| r.warnings.len()).sum();
        let total_processing_time: u64 = results.iter().map(|r| r.processing_time_ms).sum();
        let average_processing_time = if total_files > 0 {
            total_processing_time / total_files as u64
        } else {
            0
        };

        ImportStatistics {
            total_files,
            successful_imports,
            failed_imports: total_files - successful_imports,
            total_warnings,
            total_processing_time_ms: total_processing_time,
            average_processing_time_ms: average_processing_time,
        }
    }
}

/// Statistics for import operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatistics {
    pub total_files: usize,
    pub successful_imports: usize,
    pub failed_imports: usize,
    pub total_warnings: usize,
    pub total_processing_time_ms: u64,
    pub average_processing_time_ms: u64,
}

/// Configuration for chapter organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterOrganizationConfig {
    pub sorting_strategy: ChapterSortingStrategy,
    pub title_generation_strategy: ChapterTitleStrategy,
    pub include_chapter_numbers: bool,
    pub preserve_original_numbering: bool,
    pub auto_generate_titles: bool,
    pub add_chapter_metadata: bool,
    pub add_navigation_links: bool,
}

/// Strategy for sorting chapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChapterSortingStrategy {
    Filename,
    Title,
    FileSize,
    Language,
    ImportOrder,
    Custom(Vec<String>), // Custom order by filename
}

/// Strategy for generating chapter titles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChapterTitleStrategy {
    FromFilename,
    FromContent,
    NumberedOnly,
    Custom(String), // Template with placeholders
}

/// Configuration for table of contents generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocConfig {
    pub title: String,
    pub group_by_language: bool,
    pub include_links: bool,
    pub show_language_indicators: bool,
}

/// Result of chapter validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterValidationResult {
    pub is_valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub statistics: ChapterStatistics,
}

/// Statistics about chapters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterStatistics {
    pub total_chapters: usize,
    pub languages: Vec<String>,
    pub total_content_length: usize,
    pub average_chapter_length: usize,
}

impl Default for ChapterOrganizationConfig {
    fn default() -> Self {
        Self {
            sorting_strategy: ChapterSortingStrategy::Filename,
            title_generation_strategy: ChapterTitleStrategy::FromFilename,
            include_chapter_numbers: true,
            preserve_original_numbering: false,
            auto_generate_titles: true,
            add_chapter_metadata: true,
            add_navigation_links: false,
        }
    }
}

impl Default for TocConfig {
    fn default() -> Self {
        Self {
            title: "Table of Contents".to_string(),
            group_by_language: false,
            include_links: true,
            show_language_indicators: true,
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
    async fn test_new_service() {
        let service = SimplifiedDocumentImportService::new();
        assert_eq!(service.supported_formats().len(), 4);
        assert!(service.is_format_supported("test.docx"));
        assert!(service.is_format_supported("test.txt"));
        assert!(!service.is_format_supported("test.pdf"));
    }

    #[tokio::test]
    async fn test_convert_txt_to_markdown() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "This is a test document.").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "It has multiple paragraphs.").unwrap();

        let service = SimplifiedDocumentImportService::new();
        let mut messages = Vec::new();
        
        let result = service.convert_txt_to_markdown(&file_path, &mut messages).await.unwrap();
        
        assert!(result.contains("# test"));
        assert!(result.contains("This is a test document."));
        assert!(result.contains("It has multiple paragraphs."));
        assert!(!messages.is_empty());
    }

    #[tokio::test]
    async fn test_load_markdown_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "# Test Document").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "This is a markdown document.").unwrap();

        let service = SimplifiedDocumentImportService::new();
        let mut messages = Vec::new();
        
        let result = service.load_markdown_file(&file_path, &mut messages).await.unwrap();
        
        assert!(result.contains("# Test Document"));
        assert!(result.contains("This is a markdown document."));
        assert!(!messages.is_empty());
    }

    #[tokio::test]
    async fn test_import_nonexistent_file() {
        let service = SimplifiedDocumentImportService::new();
        let config = ImportConfig {
            preserve_formatting: true,
            extract_images: false,
            chapter_mode: false,
            target_language: "en".to_string(),
        };

        let result = service.import_document(Path::new("nonexistent.txt"), &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_import_unsupported_format() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.pdf");
        File::create(&file_path).unwrap();

        let service = SimplifiedDocumentImportService::new();
        let config = ImportConfig {
            preserve_formatting: true,
            extract_images: false,
            chapter_mode: false,
            target_language: "en".to_string(),
        };

        let result = service.import_document(&file_path, &config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_language_detection() {
        let service = SimplifiedDocumentImportService::new();
        
        assert_eq!(service.detect_language_from_filename("document_en.docx"), Some("en".to_string()));
        assert_eq!(service.detect_language_from_filename("document-es.txt"), Some("es".to_string()));
        assert_eq!(service.detect_language_from_filename("french_document.md"), Some("fr".to_string()));
        assert_eq!(service.detect_language_from_filename("document.docx"), None);
    }

    #[test]
    fn test_title_extraction() {
        let service = SimplifiedDocumentImportService::new();
        
        let content = "# Main Title\n\nSome content here.";
        assert_eq!(service.extract_title_from_content(content), Some("Main Title".to_string()));
        
        let content_no_title = "Some content without title.";
        assert_eq!(service.extract_title_from_content(content_no_title), None);
    }

    #[test]
    fn test_heading_detection() {
        let service = SimplifiedDocumentImportService::new();
        
        assert!(service.looks_like_heading("INTRODUCTION"));
        assert!(service.looks_like_heading("Chapter 1:"));
        assert!(!service.looks_like_heading("This is a regular paragraph with many words."));
        assert!(!service.looks_like_heading("short"));
    }

    #[test]
    fn test_list_formatting() {
        let service = SimplifiedDocumentImportService::new();
        
        let content = "* First item\n+ Second item\n- Third item";
        let improved = service.improve_list_formatting(content);
        
        let lines: Vec<&str> = improved.lines().collect();
        assert!(lines.iter().all(|line| line.starts_with("- ")));
    }

    #[test]
    fn test_import_statistics() {
        let service = SimplifiedDocumentImportService::new();
        
        let results = vec![
            DocumentImportResult {
                success: true,
                filename: "doc1.txt".to_string(),
                title: "Document 1".to_string(),
                content: "Content 1".to_string(),
                language: "en".to_string(),
                chapter_number: Some(1),
                messages: vec![],
                warnings: vec!["Warning 1".to_string()],
                processing_time_ms: 100,
            },
            DocumentImportResult {
                success: true,
                filename: "doc2.txt".to_string(),
                title: "Document 2".to_string(),
                content: "Content 2".to_string(),
                language: "en".to_string(),
                chapter_number: Some(2),
                messages: vec![],
                warnings: vec![],
                processing_time_ms: 200,
            },
        ];
        
        let stats = service.get_import_statistics(&results);
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.successful_imports, 2);
        assert_eq!(stats.failed_imports, 0);
        assert_eq!(stats.total_warnings, 1);
        assert_eq!(stats.total_processing_time_ms, 300);
        assert_eq!(stats.average_processing_time_ms, 150);
    }

    #[test]
    fn test_chapter_organization() {
        let service = SimplifiedDocumentImportService::new();
        
        let import_results = vec![
            DocumentImportResult {
                success: true,
                filename: "chapter_02.txt".to_string(),
                title: "Second Chapter".to_string(),
                content: "Content of chapter 2".to_string(),
                language: "en".to_string(),
                chapter_number: Some(2),
                messages: vec![],
                warnings: vec![],
                processing_time_ms: 100,
            },
            DocumentImportResult {
                success: true,
                filename: "chapter_01.txt".to_string(),
                title: "First Chapter".to_string(),
                content: "Content of chapter 1".to_string(),
                language: "en".to_string(),
                chapter_number: Some(1),
                messages: vec![],
                warnings: vec![],
                processing_time_ms: 100,
            },
        ];
        
        let config = ChapterOrganizationConfig {
            sorting_strategy: ChapterSortingStrategy::Filename,
            title_generation_strategy: ChapterTitleStrategy::FromFilename,
            include_chapter_numbers: true,
            preserve_original_numbering: false,
            auto_generate_titles: true,
            add_chapter_metadata: false,
            add_navigation_links: false,
        };
        
        let chapters = service.create_chapters_from_documents(&import_results, None, &config).unwrap();
        
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].chapter_number, 1); // Should be sorted by filename
        assert_eq!(chapters[0].title, "Chapter 1: Chapter 01");
        assert_eq!(chapters[1].chapter_number, 2);
        assert_eq!(chapters[1].title, "Chapter 2: Chapter 02");
    }

    #[test]
    fn test_table_of_contents_generation() {
        let service = SimplifiedDocumentImportService::new();
        
        let chapters = vec![
            Chapter {
                id: Uuid::new_v4(),
                chapter_number: 1,
                title: "Introduction".to_string(),
                content: "Intro content".to_string(),
                language: "en".to_string(),
                filename: "intro.txt".to_string(),
                created_at: Utc::now(),
            },
            Chapter {
                id: Uuid::new_v4(),
                chapter_number: 2,
                title: "Getting Started".to_string(),
                content: "Getting started content".to_string(),
                language: "en".to_string(),
                filename: "getting_started.txt".to_string(),
                created_at: Utc::now(),
            },
        ];
        
        let config = TocConfig {
            title: "Contents".to_string(),
            group_by_language: false,
            include_links: true,
            show_language_indicators: false,
        };
        
        let toc = service.create_table_of_contents(&chapters, &config);
        
        assert!(toc.contains("# Contents"));
        assert!(toc.contains("1. [Introduction]"));
        assert!(toc.contains("2. [Getting Started]"));
    }

    #[test]
    fn test_chapter_validation() {
        let service = SimplifiedDocumentImportService::new();
        
        let chapters = vec![
            Chapter {
                id: Uuid::new_v4(),
                chapter_number: 1,
                title: "Chapter 1".to_string(),
                content: "Content 1".to_string(),
                language: "en".to_string(),
                filename: "ch1.txt".to_string(),
                created_at: Utc::now(),
            },
            Chapter {
                id: Uuid::new_v4(),
                chapter_number: 3, // Gap in numbering
                title: "Chapter 3".to_string(),
                content: "Content 3".to_string(),
                language: "es".to_string(),
                filename: "ch3.txt".to_string(),
                created_at: Utc::now(),
            },
        ];
        
        let validation = service.validate_chapter_organization(&chapters);
        
        assert!(validation.is_valid);
        assert!(!validation.warnings.is_empty()); // Should warn about gap
        assert!(validation.warnings[0].contains("gap detected"));
        assert_eq!(validation.statistics.total_chapters, 2);
        assert_eq!(validation.statistics.languages.len(), 2);
    }

    #[test]
    fn test_language_organization() {
        let service = SimplifiedDocumentImportService::new();
        
        let chapters = vec![
            Chapter {
                id: Uuid::new_v4(),
                chapter_number: 1,
                title: "Chapter 1".to_string(),
                content: "English content".to_string(),
                language: "en".to_string(),
                filename: "ch1_en.txt".to_string(),
                created_at: Utc::now(),
            },
            Chapter {
                id: Uuid::new_v4(),
                chapter_number: 1,
                title: "Capítulo 1".to_string(),
                content: "Spanish content".to_string(),
                language: "es".to_string(),
                filename: "ch1_es.txt".to_string(),
                created_at: Utc::now(),
            },
        ];
        
        let organized = service.organize_chapters_by_language(&chapters);
        
        assert_eq!(organized.len(), 2);
        assert!(organized.contains_key("en"));
        assert!(organized.contains_key("es"));
        assert_eq!(organized["en"].len(), 1);
        assert_eq!(organized["es"].len(), 1);
    }
}