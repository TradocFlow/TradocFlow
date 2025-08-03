use crate::{Document, DocumentStatus, DocumentMetadata, DocumentImportRequest, DocumentImportResult, TradocumentError, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Instant;
use tempfile::NamedTempFile;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug)]
pub struct DocumentImportService {}

impl DocumentImportService {
    pub fn new() -> Self {
        Self {}
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
                format!("Unsupported file extension: {}", extension)
            ));
        }

        messages.push("Starting DOCX file processing...".to_string());

        // Read the DOCX file
        let mut file = File::open(path)
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to open file: {}", e)))?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to read file: {}", e)))?;

        messages.push("File read successfully".to_string());

        // Convert DOCX to markdown using markdownify
        let markdown_content = self.convert_docx_to_markdown(&buffer, &import_request, &mut warnings)?;
        
        messages.push("DOCX converted to markdown".to_string());

        // Create the document with multilingual support
        let document = self.create_multilingual_document(
            import_request,
            markdown_content,
            &mut messages,
        )?;

        let processing_time = start_time.elapsed().as_millis() as u64;
        messages.push(format!("Processing completed in {}ms", processing_time));

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
                format!("Unsupported file extension: {}", extension)
            ));
        }

        messages.push(format!("Starting processing of uploaded file: {}", filename));

        // Convert DOCX to markdown using markdownify
        let markdown_content = self.convert_docx_to_markdown(bytes, &import_request, &mut warnings)?;
        
        messages.push("DOCX converted to markdown".to_string());

        // Create the document with multilingual support
        let document = self.create_multilingual_document(
            import_request,
            markdown_content,
            &mut messages,
        )?;

        let processing_time = start_time.elapsed().as_millis() as u64;
        messages.push(format!("Processing completed in {}ms", processing_time));

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

    fn convert_docx_to_markdown(
        &self,
        docx_bytes: &[u8],
        _import_request: &DocumentImportRequest,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        // Create a temporary file to work with the markdownify library
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to create temp file: {}", e)))?;
        
        // Write the DOCX bytes to the temporary file
        temp_file.write_all(docx_bytes)
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to write to temp file: {}", e)))?;
        
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
                        format!("Failed to convert DOCX to markdown: {}", e)
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
                messages.push(format!("Created placeholder content for language: {}", lang));
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
            metadata: DocumentMetadata {
                languages: all_languages,
                tags: vec!["imported".to_string(), "word-document".to_string()],
                project_id: None,
                screenshots: Vec::new(),
            },
        };

        messages.push(format!("Created document with ID: {}", document.id));
        
        Ok(document)
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
