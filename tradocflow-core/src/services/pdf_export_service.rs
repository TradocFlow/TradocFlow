use genpdf::{Document, Element, elements, style::Style, fonts::FontFamily};
use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel, CodeBlockKind};
use std::path::Path;
use serde::{Serialize, Deserialize};

/// PDF Export Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfExportConfig {
    pub title: String,
    pub author: Option<String>,
    pub font_family: String,
    pub font_size: u8,
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    pub include_table_of_contents: bool,
    pub page_numbers: bool,
    pub header_text: Option<String>,
    pub footer_text: Option<String>,
}

impl Default for PdfExportConfig {
    fn default() -> Self {
        Self {
            title: "Exported Document".to_string(),
            author: None,
            font_family: "LiberationSans".to_string(),
            font_size: 12,
            margin_top: 2.0,
            margin_bottom: 2.0,
            margin_left: 2.0,
            margin_right: 2.0,
            include_table_of_contents: false,
            page_numbers: true,
            header_text: None,
            footer_text: None,
        }
    }
}

/// Progress callback for PDF export
pub type ProgressCallback = Box<dyn Fn(f32, &str) + Send>;

/// Enhanced PDF Export Service
pub struct PdfExportService {
    font_family: FontFamily<'static>,
}

#[derive(Debug)]
pub struct PdfExportError {
    pub message: String,
    pub kind: PdfExportErrorKind,
}

#[derive(Debug)]
pub enum PdfExportErrorKind {
    FontLoadError,
    ParseError,
    FileWriteError,
    ConfigurationError,
}

impl std::fmt::Display for PdfExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PdfExportError {}

impl PdfExportService {
    /// Create a new PDF export service
    pub fn new() -> Result<Self, PdfExportError> {
        // Try to load custom fonts, fallback to built-in if not available
        let font_family = genpdf::fonts::from_files("fonts", "LiberationSans", None)
            .or_else(|_| Ok(genpdf::fonts::Builtin::Helvetica.clone()))
            .map_err(|e| PdfExportError {
                message: format!("Failed to load fonts: {}", e),
                kind: PdfExportErrorKind::FontLoadError,
            })?;
        
        Ok(Self { font_family })
    }

    /// Export markdown to PDF with enhanced formatting
    pub fn export_markdown_to_pdf(
        &self,
        markdown_content: &str,
        output_path: &Path,
        config: PdfExportConfig,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<(), PdfExportError> {
        // Initialize progress
        if let Some(ref callback) = progress_callback {
            callback(0.1, "Initializing PDF export...");
        }

        // Create document
        let mut doc = Document::new(self.font_family.clone());
        doc.set_title(&config.title);
        
        if let Some(author) = &config.author {
            doc.set_author(author);
        }

        // Set page layout
        doc.set_page_size(genpdf::Size::new(210.0, 297.0)); // A4
        doc.set_margins(genpdf::Margins::trbl(
            config.margin_top.into(),
            config.margin_right.into(),
            config.margin_bottom.into(),
            config.margin_left.into(),
        ));

        if let Some(ref callback) = progress_callback {
            callback(0.2, "Parsing markdown content...");
        }

        // Parse and structure the content
        let structured_content = self.parse_markdown_structure(markdown_content)?;
        
        if let Some(ref callback) = progress_callback {
            callback(0.4, "Building document structure...");
        }

        // Note: Headers and footers are not available in genpdf 0.2.0
        // This would be available in newer versions

        if let Some(ref callback) = progress_callback {
            callback(0.5, "Adding content to document...");
        }

        // Add table of contents if requested
        if config.include_table_of_contents {
            self.add_table_of_contents(&mut doc, &structured_content)?;
        }

        // Add content
        self.add_structured_content_to_document(&mut doc, structured_content, &config)?;

        if let Some(ref callback) = progress_callback {
            callback(0.8, "Rendering PDF...");
        }

        // Render and save
        doc.render_to_file(output_path)
            .map_err(|e| PdfExportError {
                message: format!("Failed to save PDF: {}", e),
                kind: PdfExportErrorKind::FileWriteError,
            })?;

        if let Some(ref callback) = progress_callback {
            callback(1.0, "PDF export completed successfully!");
        }

        Ok(())
    }

    /// Parse markdown into structured content
    fn parse_markdown_structure(&self, markdown_content: &str) -> Result<Vec<ContentElement>, PdfExportError> {
        let parser = Parser::new(markdown_content);
        let mut elements = Vec::new();
        let mut current_element = ContentElement::Text(String::new());
        let mut stack = Vec::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    if !matches!(current_element, ContentElement::Text(ref s) if s.is_empty()) {
                        elements.push(current_element);
                    }
                    stack.push(("heading", level as usize));
                    current_element = ContentElement::Heading {
                        level: level as usize,
                        text: String::new(),
                    };
                },
                Event::End(TagEnd::Heading(_)) => {
                    stack.pop();
                    elements.push(current_element);
                    current_element = ContentElement::Text(String::new());
                },
                Event::Start(Tag::Paragraph) => {
                    if !matches!(current_element, ContentElement::Text(ref s) if s.is_empty()) {
                        elements.push(current_element);
                    }
                    current_element = ContentElement::Paragraph(String::new());
                },
                Event::End(TagEnd::Paragraph) => {
                    elements.push(current_element);
                    current_element = ContentElement::Text(String::new());
                },
                Event::Start(Tag::List(ordered)) => {
                    if !matches!(current_element, ContentElement::Text(ref s) if s.is_empty()) {
                        elements.push(current_element);
                    }
                    stack.push(if ordered.is_some() { ("ordered_list", 0) } else { ("unordered_list", 0) });
                    current_element = ContentElement::List {
                        ordered: ordered.is_some(),
                        items: Vec::new(),
                    };
                },
                Event::End(TagEnd::List(_)) => {
                    stack.pop();
                    elements.push(current_element);
                    current_element = ContentElement::Text(String::new());
                },
                Event::Start(Tag::Item) => {
                    stack.push(("list_item", 0));
                },
                Event::End(TagEnd::Item) => {
                    stack.pop();
                    if let ContentElement::List { ref mut items, .. } = current_element {
                        // This is simplified - in a real implementation, you'd collect the item content
                        items.push("List item".to_string());
                    }
                },
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                    if !matches!(current_element, ContentElement::Text(ref s) if s.is_empty()) {
                        elements.push(current_element);
                    }
                    current_element = ContentElement::CodeBlock {
                        language: if lang.is_empty() { None } else { Some(lang.to_string()) },
                        content: String::new(),
                    };
                },
                Event::End(TagEnd::CodeBlock) => {
                    elements.push(current_element);
                    current_element = ContentElement::Text(String::new());
                },
                Event::Start(Tag::Table(_)) => {
                    if !matches!(current_element, ContentElement::Text(ref s) if s.is_empty()) {
                        elements.push(current_element);
                    }
                    current_element = ContentElement::Table {
                        headers: Vec::new(),
                        rows: Vec::new(),
                    };
                },
                Event::End(TagEnd::Table) => {
                    elements.push(current_element);
                    current_element = ContentElement::Text(String::new());
                },
                Event::Text(text) => {
                    match &mut current_element {
                        ContentElement::Text(content) |
                        ContentElement::Paragraph(content) => content.push_str(&text),
                        ContentElement::Heading { text: heading_text, .. } => heading_text.push_str(&text),
                        ContentElement::CodeBlock { content, .. } => content.push_str(&text),
                        _ => {}
                    }
                },
                Event::SoftBreak => {
                    match &mut current_element {
                        ContentElement::Text(content) |
                        ContentElement::Paragraph(content) => content.push(' '),
                        ContentElement::CodeBlock { content, .. } => content.push('\n'),
                        _ => {}
                    }
                },
                Event::HardBreak => {
                    match &mut current_element {
                        ContentElement::Text(content) |
                        ContentElement::Paragraph(content) => content.push('\n'),
                        ContentElement::CodeBlock { content, .. } => content.push('\n'),
                        _ => {}
                    }
                },
                _ => {}
            }
        }

        // Add final element if not empty
        if !matches!(current_element, ContentElement::Text(ref s) if s.is_empty()) {
            elements.push(current_element);
        }

        Ok(elements)
    }

    /// Add structured content to document
    fn add_structured_content_to_document(
        &self,
        doc: &mut Document,
        elements: Vec<ContentElement>,
        config: &PdfExportConfig,
    ) -> Result<(), PdfExportError> {
        for element in elements {
            match element {
                ContentElement::Heading { level, text } => {
                    let size = match level {
                        1 => config.font_size + 6,
                        2 => config.font_size + 4,
                        3 => config.font_size + 2,
                        _ => config.font_size + 1,
                    };
                    let style = Style::new()
                        .bold()
                        .with_font_size(size);
                    
                    doc.push(elements::Paragraph::new(&text).styled(style));
                    doc.push(elements::Break::new(0.8));
                },
                ContentElement::Paragraph(text) => {
                    if !text.trim().is_empty() {
                        doc.push(elements::Paragraph::new(&text)
                            .styled(Style::new().with_font_size(config.font_size)));
                        doc.push(elements::Break::new(0.5));
                    }
                },
                ContentElement::CodeBlock { language: _, content } => {
                    let code_style = Style::new()
                        .with_font_size(config.font_size - 1);
                    doc.push(elements::Paragraph::new(&content)
                        .styled(code_style)
                        .framed());
                    doc.push(elements::Break::new(0.5));
                },
                ContentElement::List { ordered, items } => {
                    for (i, item) in items.iter().enumerate() {
                        let prefix = if ordered {
                            format!("{}. ", i + 1)
                        } else {
                            "• ".to_string()
                        };
                        doc.push(elements::Paragraph::new(&format!("{}{}", prefix, item))
                            .styled(Style::new().with_font_size(config.font_size)));
                    }
                    doc.push(elements::Break::new(0.5));
                },
                ContentElement::Table { headers: _, rows: _ } => {
                    // Simplified table handling - in a real implementation,
                    // you would create a proper table structure
                    doc.push(elements::Paragraph::new("[Table content]")
                        .styled(Style::new().italic()));
                    doc.push(elements::Break::new(0.5));
                },
                ContentElement::Text(text) => {
                    if !text.trim().is_empty() {
                        doc.push(elements::Paragraph::new(&text)
                            .styled(Style::new().with_font_size(config.font_size)));
                        doc.push(elements::Break::new(0.3));
                    }
                },
            }
        }
        Ok(())
    }

    /// Add table of contents
    fn add_table_of_contents(
        &self,
        doc: &mut Document,
        elements: &[ContentElement],
    ) -> Result<(), PdfExportError> {
        doc.push(elements::Paragraph::new("Table of Contents")
            .styled(Style::new().bold().with_font_size(16)));
        doc.push(elements::Break::new(0.8));

        for element in elements {
            if let ContentElement::Heading { level, text } = element {
                let indent = "  ".repeat(level.saturating_sub(1));
                doc.push(elements::Paragraph::new(&format!("{}{}", indent, text))
                    .styled(Style::new().with_font_size(12)));
            }
        }
        
        doc.push(elements::PageBreak::new());
        Ok(())
    }
}

/// Structured content elements
#[derive(Debug, Clone)]
enum ContentElement {
    Text(String),
    Heading { level: usize, text: String },
    Paragraph(String),
    CodeBlock { language: Option<String>, content: String },
    List { ordered: bool, items: Vec<String> },
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_pdf_export_service_creation() {
        let service = PdfExportService::new();
        assert!(service.is_ok());
    }

    #[test]
    fn test_basic_markdown_export() {
        let service = PdfExportService::new().unwrap();
        let markdown = "# Test Document\n\nThis is a test paragraph.";
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("test.pdf");
        let config = PdfExportConfig::default();

        let result = service.export_markdown_to_pdf(
            markdown,
            &output_path,
            config,
            None,
        );

        assert!(result.is_ok());
        assert!(output_path.exists());
    }
}