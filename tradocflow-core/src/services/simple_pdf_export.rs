use genpdf::{Document, Element, elements, style::Style};
use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel, CodeBlockKind};
use std::path::Path;

/// Simple PDF Export Configuration
#[derive(Debug, Clone)]
pub struct SimplePdfConfig {
    pub title: String,
    pub font_size: u8,
    pub include_toc: bool,
}

impl Default for SimplePdfConfig {
    fn default() -> Self {
        Self {
            title: "Exported Document".to_string(),
            font_size: 12,
            include_toc: false,
        }
    }
}

/// Simple PDF Export Service
pub struct SimplePdfExportService;

impl SimplePdfExportService {
    pub fn new() -> Self {
        Self
    }

    /// Export markdown to PDF with basic formatting
    pub fn export_markdown_to_pdf(
        &self,
        markdown_content: &str,
        output_path: &Path,
        config: SimplePdfConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Try to load fonts, fallback to default if not available
        let font_family = match genpdf::fonts::from_files("fonts", "LiberationSans", None) {
            Ok(family) => family,
            Err(_) => {
                eprintln!("Warning: Could not load custom fonts, using built-in Helvetica");
                genpdf::fonts::Builtin::Helvetica
            }
        };

        let mut doc = Document::new(font_family);
        doc.set_title(&config.title);

        let parser = Parser::new(markdown_content);
        let mut current_text = String::new();
        let mut in_heading = false;
        let mut heading_level = 1;
        let mut in_code_block = false;
        let mut in_list = false;
        let mut list_items = Vec::new();
        let mut current_list_item = String::new();
        let mut headings = Vec::new(); // For TOC

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    in_heading = true;
                    heading_level = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                },
                Event::End(TagEnd::Heading(_)) => {
                    if !current_text.is_empty() {
                        // Store for TOC
                        headings.push((heading_level, current_text.clone()));
                        
                        let font_size = match heading_level {
                            1 => config.font_size + 6,
                            2 => config.font_size + 4,
                            3 => config.font_size + 2,
                            _ => config.font_size + 1,
                        };
                        let style = Style::new().bold().with_font_size(font_size);
                        
                        doc.push(elements::Paragraph::new(&current_text).styled(style));
                        current_text.clear();
                    }
                    in_heading = false;
                    doc.push(elements::Break::new(0.8));
                },
                Event::Start(Tag::Paragraph) => {
                    // Start of paragraph
                },
                Event::End(TagEnd::Paragraph) => {
                    if !current_text.is_empty() && !in_heading && !in_code_block && !in_list {
                        doc.push(elements::Paragraph::new(&current_text)
                            .styled(Style::new().with_font_size(config.font_size)));
                        current_text.clear();
                        doc.push(elements::Break::new(0.5));
                    }
                },
                Event::Start(Tag::List(_)) => {
                    in_list = true;
                    list_items.clear();
                },
                Event::End(TagEnd::List(_)) => {
                    // Add all list items
                    for (i, item) in list_items.iter().enumerate() {
                        let prefix = format!("• ");
                        doc.push(elements::Paragraph::new(&format!("{}{}", prefix, item))
                            .styled(Style::new().with_font_size(config.font_size)));
                    }
                    in_list = false;
                    list_items.clear();
                    doc.push(elements::Break::new(0.5));
                },
                Event::Start(Tag::Item) => {
                    current_list_item.clear();
                },
                Event::End(TagEnd::Item) => {
                    if !current_list_item.is_empty() {
                        list_items.push(current_list_item.clone());
                        current_list_item.clear();
                    }
                },
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
                    in_code_block = true;
                },
                Event::End(TagEnd::CodeBlock) => {
                    if !current_text.is_empty() {
                        let code_style = Style::new().with_font_size(config.font_size.saturating_sub(1));
                        doc.push(elements::Paragraph::new(&current_text)
                            .styled(code_style)
                            .framed());
                        current_text.clear();
                    }
                    in_code_block = false;
                    doc.push(elements::Break::new(0.5));
                },
                Event::Text(text) => {
                    if in_list && !current_list_item.is_empty() {
                        current_list_item.push_str(&text);
                    } else if in_list {
                        current_list_item.push_str(&text);
                    } else {
                        current_text.push_str(&text);
                    }
                },
                Event::SoftBreak | Event::HardBreak => {
                    if in_code_block {
                        current_text.push('\n');
                    } else if in_list {
                        current_list_item.push(' ');
                    } else {
                        current_text.push(' ');
                    }
                },
                _ => {}
            }
        }

        // Add any remaining text
        if !current_text.is_empty() {
            doc.push(elements::Paragraph::new(&current_text)
                .styled(Style::new().with_font_size(config.font_size)));
        }

        // Add table of contents at the beginning if requested
        if config.include_toc && !headings.is_empty() {
            // Add TOC to main document
            doc.push(elements::Paragraph::new("Table of Contents")
                .styled(Style::new().bold().with_font_size(config.font_size + 4)));
            doc.push(elements::Break::new(0.8));

            for (level, heading) in &headings {
                let indent_count = if *level > 1 { *level - 1 } else { 0 };
                let indent = "  ".repeat(indent_count);
                doc.push(elements::Paragraph::new(&format!("{}{}", indent, heading))
                    .styled(Style::new().with_font_size(config.font_size)));
            }
            doc.push(elements::PageBreak::new());
        }

        // Render and save the PDF
        doc.render_to_file(output_path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_simple_pdf_export() {
        let service = SimplePdfExportService::new();
        let markdown = "# Test Document\n\nThis is a test paragraph.\n\n## Section\n\n- Item 1\n- Item 2\n\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("test.pdf");
        let config = SimplePdfConfig::default();

        let result = service.export_markdown_to_pdf(markdown, &output_path, config);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }
}