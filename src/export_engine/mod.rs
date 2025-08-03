use crate::{Document, ScreenshotReference, Result};
use comrak::{markdown_to_html, ComrakOptions};
use genpdf::{elements, fonts};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use toml::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub include_screenshots: bool,
    pub template: Option<String>,
    pub css_file: Option<String>,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Html,
    Pdf,
    Both,
}

pub struct ExportEngine {
    comrak_options: ComrakOptions<'static>,
    fragments: HashMap<String, String>,
}

impl ExportEngine {
    pub fn new() -> Self {
        let mut options = ComrakOptions::default();
        options.extension.strikethrough = true;
        options.extension.tagfilter = true;
        options.extension.table = true;
        options.extension.autolink = true;
        options.extension.tasklist = true;
        options.extension.superscript = true;
        options.extension.header_ids = Some("user-content-".to_string());
        options.extension.footnotes = true;
        options.extension.description_lists = true;
        options.extension.front_matter_delimiter = Some("+++".to_string());

        let fragments = Self::load_fragments().unwrap_or_default();

        Self {
            comrak_options: options,
            fragments,
        }
    }

    fn load_fragments() -> Result<HashMap<String, String>> {
        let fragments_content = fs::read_to_string("fragments.toml")?;
        let fragments_value: Value = toml::from_str(&fragments_content)?;
        let mut fragments_map = HashMap::new();

        if let Some(fragments_table) = fragments_value.get("fragments").and_then(|v| v.as_table()) {
            for (key, value) in fragments_table {
                if let Some(str_value) = value.as_str() {
                    fragments_map.insert(key.clone(), str_value.to_string());
                }
            }
        }

        Ok(fragments_map)
    }

    fn process_fragments(&self, content: &str) -> String {
        let mut processed_content = content.to_string();
        for (key, value) in &self.fragments {
            let placeholder = format!("§{{{}}}", key);
            processed_content = processed_content.replace(&placeholder, value);
        }
        processed_content
    }

    pub async fn export_document(
        &self,
        document: &Document,
        config: &ExportConfig,
    ) -> Result<HashMap<String, Vec<u8>>> {
        let mut results = HashMap::new();

        for language in &config.languages {
            if let Some(content) = document.content.get(language) {
                let content_with_fragments = self.process_fragments(content);
                let processed_content = self.process_screenshots(&content_with_fragments, &document.metadata.screenshots, language).await?;

                match config.format {
                    ExportFormat::Html => {
                        let html = self.generate_html(&processed_content, config)?;
                        results.insert(format!("{}.html", language), html.into_bytes());
                    }
                    ExportFormat::Pdf => {
                        let pdf = self.generate_pdf(&processed_content, config)?;
                        results.insert(format!("{}.pdf", language), pdf);
                    }
                    ExportFormat::Both => {
                        let html = self.generate_html(&processed_content, config)?;
                        let pdf = self.generate_pdf(&processed_content, config)?;
                        results.insert(format!("{}.html", language), html.into_bytes());
                        results.insert(format!("{}.pdf", language), pdf);
                    }
                }
            }
        }

        Ok(results)
    }

    fn extract_language_variable(&self, content: &str) -> Option<String> {
        // Look for <!-- lang: [code] --> at the beginning of the document
        let lines: Vec<&str> = content.lines().take(10).collect(); // Check first 10 lines
        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("<!-- lang:") && trimmed.ends_with("-->") {
                let lang_part = trimmed.strip_prefix("<!-- lang:").unwrap().strip_suffix("-->").unwrap();
                let lang_code = lang_part.trim();
                if !lang_code.is_empty() {
                    return Some(lang_code.to_string());
                }
            }
        }
        None
    }

    async fn process_screenshots(
        &self,
        content: &str,
        screenshots: &[ScreenshotReference],
        fallback_language: &str,
    ) -> Result<String> {
        let mut processed = content.to_string();
        
        // Extract language from document variable or use fallback
        let document_language = self.extract_language_variable(content).unwrap_or_else(|| fallback_language.to_string());

        for screenshot in screenshots {
            // Use document language for screenshot matching instead of screenshot.language
            if screenshot.language == document_language {
                let placeholder = format!("{{screenshot:{}}}", screenshot.id);
                if processed.contains(&placeholder) {
                    let img_tag = format!(
                        "![Screenshot {}](screenshots/{}/{}.svg)",
                        screenshot.id, document_language, screenshot.id
                    );
                    processed = processed.replace(&placeholder, &img_tag);
                }
            }
        }

        // Remove the language variable line from processed content
        if let Some(_) = self.extract_language_variable(&processed) {
            let lines: Vec<&str> = processed.lines().collect();
            let mut filtered_lines = Vec::new();
            for line in lines {
                let trimmed = line.trim();
                if !(trimmed.starts_with("<!-- lang:") && trimmed.ends_with("-->")) {
                    filtered_lines.push(line);
                }
            }
            processed = filtered_lines.join("\n");
        }

        Ok(processed)
    }

    fn generate_html(&self, content: &str, config: &ExportConfig) -> Result<String> {
        let html_body = markdown_to_html(content, &self.comrak_options);

        let css = if let Some(css_file) = &config.css_file {
            std::fs::read_to_string(css_file)?
        } else {
            include_str!("default.css").to_string()
        };

        let full_html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document</title>
    <style>{}</style>
</head>
<body>
    <div class="document-content">
        {}
    </div>
</body>
</html>"#,
            css, html_body
        );

        Ok(full_html)
    }

    fn generate_pdf(&self, content: &str, _config: &ExportConfig) -> Result<Vec<u8>> {
        // Load fonts or return error
        let font_family = fonts::from_files("fonts", "LiberationSans", None)
            .map_err(|e| crate::TradocumentError::Pdf(format!("Font loading failed: {}", e)))?;

        let mut doc = genpdf::Document::new(font_family);
        doc.set_title("Tradocument Review");

        // Convert markdown to HTML then to plain text for PDF
        let html_content = markdown_to_html(content, &self.comrak_options);
        
        // Basic HTML stripping for simple text content
        let mut text_content = html_content;
        let replacements = [
            ("<h1>", "\n\n"), ("</h1>", "\n"),
            ("<h2>", "\n"), ("</h2>", "\n"),
            ("<h3>", "\n"), ("</h3>", "\n"),
            ("<p>", ""), ("</p>", "\n"),
            ("<strong>", ""), ("</strong>", ""),
            ("<em>", ""), ("</em>", ""),
            ("<code>", ""), ("</code>", ""),
            ("&amp;", "&"), ("&lt;", "<"), ("&gt;", ">"), ("&quot;", "\""),
        ];
        
        for (from, to) in &replacements {
            text_content = text_content.replace(from, to);
        }

        // Add content as paragraphs
        for paragraph in text_content.split("\n\n") {
            let trimmed = paragraph.trim();
            if !trimmed.is_empty() {
                doc.push(elements::Paragraph::new(trimmed));
            }
        }

        let mut pdf_bytes = Vec::new();
        doc.render(&mut pdf_bytes).map_err(|e| crate::TradocumentError::Pdf(e.to_string()))?;
        Ok(pdf_bytes)
    }

    pub async fn generate_screenshots(
        &self,
        document: &Document,
        language: &str,
    ) -> Result<Vec<ScreenshotReference>> {
        let mut screenshots = Vec::new();

        if let Some(project_id) = &document.metadata.project_id {
            // Use screenshot_creator to generate screenshots for this language
            // This would integrate with the bell_tower_controller web interface
            let screenshot_ref = ScreenshotReference {
                id: format!("main_screen_{}", language),
                language: language.to_string(),
                screen_config: format!(
                    r#"{{"project_id": "{}", "language": "{}"}}"#,
                    project_id,
                    language
                ),
                generated_at: Some(chrono::Utc::now()),
            };
            screenshots.push(screenshot_ref);
        }

        Ok(screenshots)
    }
}