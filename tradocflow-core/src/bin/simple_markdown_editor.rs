use slint::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use std::thread;

// Import document processing services
use tradocflow_core::services::{
    ThreadSafeDocumentProcessor, DocumentProcessingConfig, ImportProgressInfo, ImportStage
};

// Import the generated Slint components
use tradocflow_core::MainWindow;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuadLayout {
    Single,
    Horizontal,
    Vertical,
    Quad,
}

#[derive(Debug, Clone)]
pub struct PanelInfo {
    pub id: String,
    pub file_path: String,
    pub content: String,
    pub view_mode: String, // "markdown" | "presentation"  
    pub is_modified: bool,
    pub cursor_position: i32,
}

#[derive(Debug, Clone)]
pub struct PdfExportConfig {
    // Paper settings
    pub paper_format: String, // "A4", "Letter", "Legal", "A3", "A5", "Custom"
    pub orientation: String,   // "Portrait", "Landscape"
    pub custom_width: i32,
    pub custom_height: i32,
    
    // Margins (in mm)
    pub margin_top: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,
    pub margin_right: i32,
    
    // Font settings
    pub base_font: String,
    pub font_size: i32,
    pub line_height: i32,
    
    // Content options
    pub include_toc: bool,
    pub include_page_numbers: bool,
    pub include_headers_footers: bool,
    pub header_text: String,
    pub footer_text: String,
    pub syntax_highlighting: bool,
    pub preserve_code_formatting: bool,
    
    // Link handling
    pub link_handling: String, // "Preserve", "RemoveFormatting", "ConvertToFootnotes"
    
    // Image quality
    pub image_quality: String, // "Low", "Medium", "High", "Original"
    
    // Metadata
    pub document_title: String,
    pub document_author: String,
    pub document_subject: String,
}

#[derive(Debug, Clone)]
pub struct PdfExportProgress {
    pub visible: bool,
    pub stage: String,
    pub progress_percent: i32,
    pub current_item: String,
    pub items_completed: i32,
    pub total_items: i32,
    pub message: String,
    pub warnings: Vec<String>,
    pub can_cancel: bool,
}

#[derive(Debug, Clone)]
struct PanelState {
    file_path: String,
    content: String,
    view_mode: String,
    is_modified: bool,
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    
    // Initialize document processing service
    let document_processor = match ThreadSafeDocumentProcessor::new() {
        Ok(processor) => Some(processor),
        Err(e) => {
            eprintln!("⚠️  Warning: Document processing service failed to initialize: {}", e);
            eprintln!("   Word import functionality will be limited");
            None
        }
    };
    
    // PDF export service temporarily disabled due to API compatibility issues
    // Enhanced PDF export will be implemented after resolving genpdf dependencies
    
    // Panel state management  
    let panel_states = Rc::new(RefCell::new(vec![
        PanelState {
            file_path: String::new(),
            content: "# Welcome to Simple Markdown Editor\n\nStart editing your markdown here...".to_string(),
            view_mode: "markdown".to_string(),
            is_modified: false,
        },
        PanelState {
            file_path: String::new(),
            content: "# Panel 2\n\nSecond panel for additional content...".to_string(),
            view_mode: "markdown".to_string(),
            is_modified: false,
        },
        PanelState {
            file_path: String::new(),
            content: "# Panel 3\n\nThird panel for even more content...".to_string(),
            view_mode: "markdown".to_string(),
            is_modified: false,
        },
        PanelState {
            file_path: String::new(),
            content: "# Panel 4\n\nFourth panel completes the quad layout...".to_string(),
            view_mode: "markdown".to_string(),
            is_modified: false,
        },
    ]));

    // Setup initial content
    ui.set_document_content("# Welcome to Simple Markdown Editor\n\nStart editing your markdown here...".into());
    ui.set_current_mode("markdown".into());
    ui.set_current_layout("single".into());
    ui.set_status_message("Ready - Simple Markdown Editor".into());
    ui.set_status_type("info".into());

    // File operations
    let ui_handle = ui.as_weak();
    ui.on_file_new(move || {
        let ui = ui_handle.unwrap();
        ui.set_document_content("# New Document\n\nStart typing your markdown here...".into());
        ui.set_status_message("New document created".into());
        ui.set_status_type("success".into());
        println!("📄 New document created");
    });

    // Open file
    let ui_handle = ui.as_weak();
    ui.on_file_open(move || {
        let ui = ui_handle.unwrap();
        ui.set_status_message("Opening file...".into());
        ui.set_status_type("info".into());
        println!("📁 Open file requested");
        // File dialog would go here
    });

    // Save file
    let ui_handle = ui.as_weak();
    ui.on_file_save(move || {
        let ui = ui_handle.unwrap();
        ui.set_status_message("File saved".into());
        ui.set_status_type("success".into());
        println!("💾 Save file requested");
        // Save logic would go here
    });

    // Run the application
    ui.run()
}

// Basic PDF export function using genpdf and pulldown-cmark (fallback)
fn export_markdown_to_pdf_basic(markdown_content: &str, output_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use genpdf::{Document, Element, style::Style};
    use pulldown_cmark::{Parser, Event, Tag, HeadingLevel, CodeBlockKind};
    
    let mut doc = Document::new(genpdf::fonts::from_files("fonts", "LiberationSans", None)?);
    doc.set_title("Exported Markdown Document");
    
    let parser = Parser::new(markdown_content);
    let mut current_text = String::new();
    let mut in_heading = false;
    let mut heading_level = 1;
    let mut in_code_block = false;
    
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
            Event::End(_) if in_heading => {
                if !current_text.is_empty() {
                    let style = match heading_level {
                        1 => Style::new().bold().with_font_size(18),
                        2 => Style::new().bold().with_font_size(16),
                        3 => Style::new().bold().with_font_size(14),
                        _ => Style::new().bold().with_font_size(12),
                    };
                    doc.push(genpdf::elements::Paragraph::new(&current_text).styled(style));
                    current_text.clear();
                }
                in_heading = false;
                doc.push(genpdf::elements::Break::new(0.5));
            },
            Event::Start(Tag::Paragraph) => {
                // Start of paragraph  
            },
            Event::End(_) if !in_heading && !in_code_block => {
                if !current_text.is_empty() && !in_heading && !in_code_block {
                    doc.push(genpdf::elements::Paragraph::new(&current_text));
                    current_text.clear();
                    doc.push(genpdf::elements::Break::new(0.3));
                }
            },
            Event::Text(text) => {
                current_text.push_str(&text);
            },
            _ => {}
        }
    }
    
    // Render and save the PDF
    doc.render_to_file(output_path)?;
    
    Ok(())
}
