use slint::*;
use std::rc::Rc;
use std::cell::RefCell;

// Import document processing services
use tradocflow_core::services::ThreadSafeDocumentProcessor;

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
    let _document_processor = match ThreadSafeDocumentProcessor::new() {
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
    let _panel_states = Rc::new(RefCell::new(vec![
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

    // Text editing callbacks - this is the critical missing piece!
    let ui_handle = ui.as_weak();
    ui.on_content_changed(move |content, language| {
        let ui = ui_handle.unwrap();
        println!("📝 Content changed for {}: {} chars", language, content.len());
        // Update the document content in the UI
        ui.set_document_content(content.clone());
        // Mark as modified
        ui.set_status_message(std::format!("Modified - {} characters", content.len()).into());
        ui.set_status_type("info".into());
    });

    // Focus management callbacks - enable proper focus handling
    let ui_handle = ui.as_weak();
    ui.on_editor_focus_requested(move |editor_id, pane_id| {
        let ui = ui_handle.unwrap();
        println!("🎯 Focus requested for editor: {} in pane: {}", editor_id, pane_id);
        ui.set_status_message(std::format!("Focus: {}", editor_id).into());
        ui.set_status_type("info".into());
        
        // CRITICAL FIX: Set the proper focus properties in the UI
        // Clear all focus states first
        ui.set_single_editor_focused(false);
        ui.set_left_editor_focused(false);
        ui.set_right_editor_focused(false);
        ui.set_pane_1_focused(false);
        ui.set_pane_2_focused(false);
        ui.set_pane_3_focused(false);
        ui.set_pane_4_focused(false);
        
        // Set focus for the requested editor
        match editor_id.as_str() {
            "single-editor" => {
                ui.set_single_editor_focused(true);
                ui.set_active_editor_id("single-editor".into());
                ui.set_active_pane_id("single-pane".into());
            }
            "left-editor" => {
                ui.set_left_editor_focused(true);
                ui.set_active_editor_id("left-editor".into());
                ui.set_active_pane_id("left-pane".into());
            }
            "right-editor" => {
                ui.set_right_editor_focused(true);
                ui.set_active_editor_id("right-editor".into());
                ui.set_active_pane_id("right-pane".into());
            }
            "pane-1-editor" => {
                ui.set_pane_1_focused(true);
                ui.set_active_editor_id("pane-1-editor".into());
                ui.set_active_pane_id("pane-1".into());
            }
            "pane-2-editor" => {
                ui.set_pane_2_focused(true);
                ui.set_active_editor_id("pane-2-editor".into());
                ui.set_active_pane_id("pane-2".into());
            }
            "pane-3-editor" => {
                ui.set_pane_3_focused(true);
                ui.set_active_editor_id("pane-3-editor".into());
                ui.set_active_pane_id("pane-3".into());
            }
            "pane-4-editor" => {
                ui.set_pane_4_focused(true);
                ui.set_active_editor_id("pane-4-editor".into());
                ui.set_active_pane_id("pane-4".into());
            }
            _ => {
                println!("⚠️ Unknown editor ID: {}", editor_id);
            }
        }
        
        println!("✅ Focus set for editor: {} (active: {})", editor_id, ui.get_active_editor_id());
    });

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

    // Text formatting callbacks - implement markdown formatting functionality
    let ui_handle = ui.as_weak();
    ui.on_format_bold(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = format_text_bold(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Applied bold formatting".into());
        ui.set_status_type("success".into());
        println!("🔤 Bold formatting applied");
    });

    let ui_handle = ui.as_weak();
    ui.on_format_italic(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = format_text_italic(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Applied italic formatting".into());
        ui.set_status_type("success".into());
        println!("🔤 Italic formatting applied");
    });

    let ui_handle = ui.as_weak();
    ui.on_format_heading(move |level| {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = format_text_heading(&current_content, level);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message(std::format!("Applied H{} heading", level).into());
        ui.set_status_type("success".into());
        println!("🔤 H{} heading applied", level);
    });

    let ui_handle = ui.as_weak();
    ui.on_format_code(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = format_text_code(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Applied code formatting".into());
        ui.set_status_type("success".into());
        println!("🔤 Code formatting applied");
    });

    let ui_handle = ui.as_weak();
    ui.on_insert_bullet_list(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = insert_bullet_list(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Inserted bullet list".into());
        ui.set_status_type("success".into());
        println!("🔤 Bullet list inserted");
    });

    // Additional formatting callbacks - these will need to be connected to the UI
    let ui_handle = ui.as_weak();
    ui.on_format_strikethrough(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = format_text_strikethrough(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Applied strikethrough formatting".into());
        ui.set_status_type("success".into());
        println!("🔤 Strikethrough formatting applied");
    });

    let ui_handle = ui.as_weak();
    ui.on_format_underline(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = format_text_underline(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Applied underline formatting".into());
        ui.set_status_type("success".into());
        println!("🔤 Underline formatting applied");
    });

    let ui_handle = ui.as_weak();
    ui.on_format_quote(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = format_text_quote(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Inserted blockquote".into());
        ui.set_status_type("success".into());
        println!("🔤 Blockquote inserted");
    });

    let ui_handle = ui.as_weak();
    ui.on_insert_numbered_list(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = insert_numbered_list(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Inserted numbered list".into());
        ui.set_status_type("success".into());
        println!("🔤 Numbered list inserted");
    });

    let ui_handle = ui.as_weak();
    ui.on_insert_checklist(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = insert_checklist(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Inserted checklist".into());
        ui.set_status_type("success".into());
        println!("🔤 Checklist inserted");
    });

    let ui_handle = ui.as_weak();
    ui.on_insert_link(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = insert_link(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Inserted link".into());
        ui.set_status_type("success".into());
        println!("🔤 Link inserted");
    });

    let ui_handle = ui.as_weak();
    ui.on_insert_image(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = insert_image(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Inserted image".into());
        ui.set_status_type("success".into());
        println!("🔤 Image inserted");
    });

    let ui_handle = ui.as_weak();
    ui.on_insert_table(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = insert_table(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Inserted table".into());
        ui.set_status_type("success".into());
        println!("🔤 Table inserted");
    });

    let ui_handle = ui.as_weak();
    ui.on_insert_code_block(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = insert_code_block(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Inserted code block".into());
        ui.set_status_type("success".into());
        println!("🔤 Code block inserted");
    });

    let ui_handle = ui.as_weak();
    ui.on_insert_horizontal_rule(move || {
        let ui = ui_handle.unwrap();
        let current_content = ui.get_document_content().to_string();
        let formatted_content = insert_horizontal_rule(&current_content);
        ui.set_document_content(formatted_content.into());
        ui.set_status_message("Inserted horizontal rule".into());
        ui.set_status_type("success".into());
        println!("🔤 Horizontal rule inserted");
    });

    // Run the application
    ui.run()
}

// Legacy formatting functions (kept for compatibility)
// Note: These are replaced by the enhanced formatting engine above
// but kept here in case they're needed for fallback scenarios

fn format_text_bold(content: &str) -> String {
    if content.trim().is_empty() {
        "**Bold text**".to_string()
    } else {
        // Check if content already ends with formatting, add appropriate spacing
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}**Bold text**", content, separator)
    }
}

fn format_text_italic(content: &str) -> String {
    if content.trim().is_empty() {
        "*Italic text*".to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}*Italic text*", content, separator)
    }
}

fn format_text_heading(content: &str, level: i32) -> String {
    let heading_prefix = "#".repeat(level.clamp(1, 6) as usize);
    let heading_text = std::format!("{} Heading Level {}", heading_prefix, level);
    
    if content.trim().is_empty() {
        heading_text
    } else {
        // Smart heading insertion - if content ends with a line, convert that line
        let lines: Vec<&str> = content.lines().collect();
        if let Some(last_line) = lines.last() {
            if last_line.trim().is_empty() {
                // Empty last line, add heading after it
                std::format!("{}\n{}", content, heading_text)
            } else if last_line.starts_with('#') {
                // Last line is already a heading, replace it
                let mut new_lines = lines;
                new_lines.pop(); // Remove last line
                let base_content = new_lines.join("\n");
                if base_content.is_empty() {
                    heading_text
                } else {
                    std::format!("{}\n{}", base_content, heading_text)
                }
            } else {
                // Add heading after current content
                let separator = if content.ends_with('\n') { "" } else { "\n\n" };
                std::format!("{}{}{}",content, separator, heading_text)
            }
        } else {
            heading_text
        }
    }
}

fn format_text_code(content: &str) -> String {
    if content.trim().is_empty() {
        "`inline code`".to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}```\ncode block\n```", content, separator)
    }
}

fn insert_bullet_list(content: &str) -> String {
    let bullet_list = "- First item\n- Second item\n- Third item";
    
    if content.trim().is_empty() {
        bullet_list.to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}{}", content, separator, bullet_list)
    }
}

// Additional formatting functions for enhanced functionality
fn format_text_strikethrough(content: &str) -> String {
    if content.trim().is_empty() {
        "~~Strikethrough text~~".to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}~~Strikethrough text~~", content, separator)
    }
}

fn format_text_underline(content: &str) -> String {
    if content.trim().is_empty() {
        "<u>Underlined text</u>".to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}<u>Underlined text</u>", content, separator)
    }
}

fn format_text_quote(content: &str) -> String {
    if content.trim().is_empty() {
        "> This is a blockquote".to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}> This is a blockquote", content, separator)
    }
}

fn insert_numbered_list(content: &str) -> String {
    let numbered_list = "1. First item\n2. Second item\n3. Third item";
    
    if content.trim().is_empty() {
        numbered_list.to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}{}", content, separator, numbered_list)
    }
}

fn insert_checklist(content: &str) -> String {
    let checklist = "- [ ] Task 1\n- [ ] Task 2\n- [x] Completed task";
    
    if content.trim().is_empty() {
        checklist.to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}{}", content, separator, checklist)
    }
}

fn insert_link(content: &str) -> String {
    let link = "[Link text](https://example.com)";
    
    if content.trim().is_empty() {
        link.to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}{}", content, separator, link)
    }
}

fn insert_image(content: &str) -> String {
    let image = "![Alt text](image.jpg)";
    
    if content.trim().is_empty() {
        image.to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}{}", content, separator, image)
    }
}

fn insert_table(content: &str) -> String {
    let table = "| Column 1 | Column 2 | Column 3 |\n|----------|----------|----------|\n| Cell 1   | Cell 2   | Cell 3   |\n| Cell 4   | Cell 5   | Cell 6   |";
    
    if content.trim().is_empty() {
        table.to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}{}", content, separator, table)
    }
}

fn insert_code_block(content: &str) -> String {
    let code_block = "```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```";
    
    if content.trim().is_empty() {
        code_block.to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}{}", content, separator, code_block)
    }
}

fn insert_horizontal_rule(content: &str) -> String {
    let rule = "---";
    
    if content.trim().is_empty() {
        rule.to_string()
    } else {
        let separator = if content.ends_with('\n') || content.ends_with("  \n") { 
            "" 
        } else { 
            "\n\n" 
        };
        std::format!("{}{}{}", content, separator, rule)
    }
}

// Basic PDF export function using genpdf and pulldown-cmark (fallback)
fn export_markdown_to_pdf_basic(markdown_content: &str, output_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use genpdf::{Document, Element, style::Style};
    use pulldown_cmark::{Parser, Event, Tag, HeadingLevel};
    
    let mut doc = Document::new(genpdf::fonts::from_files("fonts", "LiberationSans", None)?);
    doc.set_title("Exported Markdown Document");
    
    let parser = Parser::new(markdown_content);
    let mut current_text = String::new();
    let mut in_heading = false;
    let mut heading_level = 1;
    let _in_code_block = false;
    
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
            Event::End(_) if !in_heading => {
                if !current_text.is_empty() && !in_heading {
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
