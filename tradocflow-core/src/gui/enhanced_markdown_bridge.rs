use crate::services::markdown_service::{
    MarkdownService, RenderedMarkdown, MarkdownElement as ServiceMarkdownElement,
    FormatType, ValidationError, ValidationErrorType
};
use anyhow::Result;
use slint::{ModelRc, SharedString};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::fs;

// Document statistics for the UI
#[derive(Clone, Debug, Default)]
pub struct DocumentStats {
    pub word_count: i32,
    pub character_count: i32,
    pub heading_count: i32,
    pub link_count: i32,
    pub image_count: i32,
    pub table_count: i32,
    pub list_count: i32,
}

// Enhanced Slint types for the editor
#[derive(Clone, Debug)]
pub struct MarkdownElement {
    pub element_type: SharedString,
    pub content: SharedString,
    pub start_line: i32,
    pub start_col: i32,
    pub end_line: i32,
    pub end_col: i32,
    pub editable: bool,
    pub element_id: SharedString,
}

#[derive(Clone, Debug)]
pub struct RenderedContent {
    pub html: SharedString,
    pub elements: ModelRc<MarkdownElement>,
    pub word_count: i32,
    pub heading_count: i32,
}

// Validation error for Slint UI
#[derive(Clone, Debug)]
pub struct UIValidationError {
    pub line: i32,
    pub column: i32,
    pub message: SharedString,
    pub error_type: SharedString,
}

// Syntax element for highlighting
#[derive(Clone, Debug)]
pub struct UISyntaxElement {
    pub element_type: SharedString,
    pub start_line: i32,
    pub start_col: i32,
    pub end_line: i32,
    pub end_col: i32,
    pub valid: bool,
}

// Enhanced bridge with full editor functionality
pub struct EnhancedMarkdownBridge {
    service: MarkdownService,
    current_content: std::sync::Mutex<String>,
    current_filename: std::sync::Mutex<Option<String>>,
    document_modified: std::sync::Mutex<bool>,
    cursor_position: std::sync::Mutex<(i32, i32)>, // line, column
    selection_range: std::sync::Mutex<(i32, i32)>, // start, length
}

impl EnhancedMarkdownBridge {
    /// Create a new enhanced markdown editor bridge
    pub fn new() -> Self {
        Self {
            service: MarkdownService::new(),
            current_content: std::sync::Mutex::new(String::new()),
            current_filename: std::sync::Mutex::new(None),
            document_modified: std::sync::Mutex::new(false),
            cursor_position: std::sync::Mutex::new((0, 0)),
            selection_range: std::sync::Mutex::new((0, 0)),
        }
    }
    
    /// Render markdown content to structured format for UI
    pub fn render_markdown(&self, markdown: &str) -> Result<RenderedContent> {
        let rendered = self.service.parse_to_elements(markdown)?;
        Ok(self.convert_to_slint_rendered(&rendered))
    }
    
    /// Convert service RenderedMarkdown to Slint RenderedContent
    fn convert_to_slint_rendered(&self, rendered: &RenderedMarkdown) -> RenderedContent {
        let elements: Vec<MarkdownElement> = rendered.elements
            .iter()
            .map(|element| self.convert_element_to_slint(element))
            .collect();
        
        RenderedContent {
            html: SharedString::from(rendered.html.clone()),
            elements: ModelRc::from(elements.as_slice()),
            word_count: rendered.metadata.word_count as i32,
            heading_count: rendered.metadata.heading_count as i32,
        }
    }
    
    /// Convert service MarkdownElement to Slint MarkdownElement
    fn convert_element_to_slint(&self, element: &ServiceMarkdownElement) -> MarkdownElement {
        MarkdownElement {
            element_type: SharedString::from(element.element_type.clone()),
            content: SharedString::from(element.content.clone()),
            start_line: element.position.start_line as i32,
            start_col: element.position.start_col as i32,
            end_line: element.position.end_line as i32,
            end_col: element.position.end_col as i32,
            editable: element.editable,
            element_id: SharedString::from(format!("{}-{}", element.element_type, element.position.start_line)),
        }
    }
    
    /// Update content and mark as modified
    pub fn update_content(&self, content: &str) -> Result<()> {
        let mut current_content = self.current_content.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock content: {}", e)
        })?;
        *current_content = content.to_string();
        
        let mut modified = self.document_modified.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock modified flag: {}", e)
        })?;
        *modified = true;
        
        Ok(())
    }
    
    /// Get current content
    pub fn get_content(&self) -> Result<String> {
        let content = self.current_content.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock content: {}", e)
        })?;
        Ok(content.clone())
    }
    
    /// Calculate comprehensive document statistics
    pub fn calculate_statistics(&self, content: &str) -> DocumentStats {
        let rendered = self.service.parse_to_elements(content).unwrap_or_else(|_| {
            // Fallback to basic parsing
            self.basic_stats_calculation(content)
        });
        
        let list_count = rendered.elements.iter()
            .filter(|e| e.element_type.contains("list") || e.element_type == "list_item")
            .count() as i32;
        
        DocumentStats {
            word_count: rendered.metadata.word_count as i32,
            character_count: content.chars().count() as i32,
            heading_count: rendered.metadata.heading_count as i32,
            link_count: rendered.metadata.link_count as i32,
            image_count: rendered.metadata.image_count as i32,
            table_count: rendered.metadata.table_count as i32,
            list_count,
        }
    }
    
    /// Basic statistics calculation fallback
    fn basic_stats_calculation(&self, content: &str) -> RenderedMarkdown {
        use crate::services::markdown_service::{MarkdownMetadata, RenderedMarkdown};
        
        let word_count = content.split_whitespace().count();
        let heading_count = content.lines()
            .filter(|line| line.trim_start().starts_with('#'))
            .count();
        
        RenderedMarkdown {
            html: String::new(),
            elements: Vec::new(),
            metadata: MarkdownMetadata {
                word_count,
                heading_count,
                link_count: 0,
                image_count: 0,
                table_count: 0,
            },
        }
    }
    
    /// File operations
    pub fn new_document(&self) -> Result<()> {
        self.update_content("")?;
        
        let mut filename = self.current_filename.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock filename: {}", e)
        })?;
        *filename = None;
        
        let mut modified = self.document_modified.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock modified flag: {}", e)
        })?;
        *modified = false;
        
        Ok(())
    }
    
    /// Open file
    pub fn open_file(&self, filepath: &str) -> Result<String> {
        let content = fs::read_to_string(filepath)?;
        self.update_content(&content)?;
        
        let mut filename = self.current_filename.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock filename: {}", e)
        })?;
        *filename = Some(filepath.to_string());
        
        let mut modified = self.document_modified.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock modified flag: {}", e)
        })?;
        *modified = false;
        
        Ok(Path::new(filepath).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string())
    }
    
    /// Save file
    pub fn save_file(&self, filepath: Option<&str>) -> Result<String> {
        let content = self.get_content()?;
        
        let save_path = if let Some(path) = filepath {
            path.to_string()
        } else {
            let filename_guard = self.current_filename.lock().map_err(|e| {
                anyhow::anyhow!("Failed to lock filename: {}", e)
            })?;
            filename_guard.clone().ok_or_else(|| {
                anyhow::anyhow!("No filename specified")
            })?
        };
        
        fs::write(&save_path, content)?;
        
        let mut filename = self.current_filename.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock filename: {}", e)
        })?;
        *filename = Some(save_path.clone());
        
        let mut modified = self.document_modified.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock modified flag: {}", e)
        })?;
        *modified = false;
        
        Ok(Path::new(&save_path).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string())
    }
    
    /// Export to HTML
    pub fn export_html(&self, filepath: &str) -> Result<()> {
        let content = self.get_content()?;
        let html = self.service.render_to_html(&content)?;
        
        let full_html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Markdown Document</title>
    <style>
        body {{ 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 800px;
            margin: 0 auto;
            padding: 2rem;
        }}
        h1, h2, h3, h4, h5, h6 {{ color: #2563eb; }}
        code {{ 
            background: #f3f4f6;
            padding: 2px 4px;
            border-radius: 3px;
            font-size: 0.9em;
        }}
        pre {{ 
            background: #f3f4f6;
            padding: 1rem;
            border-radius: 6px;
            overflow-x: auto;
        }}
        blockquote {{ 
            border-left: 4px solid #e5e7eb;
            margin: 0;
            padding-left: 1rem;
            color: #6b7280;
        }}
        table {{ 
            width: 100%;
            border-collapse: collapse;
            margin: 1rem 0;
        }}
        th, td {{ 
            border: 1px solid #e5e7eb;
            padding: 0.5rem;
            text-align: left;
        }}
        th {{ background: #f9fafb; }}
    </style>
</head>
<body>
{}
</body>
</html>"#,
            html
        );
        
        fs::write(filepath, full_html)?;
        Ok(())
    }
    
    /// Export to PDF (placeholder - would need PDF generation library)
    pub fn export_pdf(&self, filepath: &str) -> Result<()> {
        // For now, export as HTML with PDF-friendly styling
        let content = self.get_content()?;
        let html = self.service.render_to_html(&content)?;
        
        let pdf_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Markdown Document</title>
    <style>
        @media print {{
            body {{ margin: 0; }}
            @page {{ margin: 2cm; }}
        }}
        body {{ 
            font-family: 'Times New Roman', serif;
            line-height: 1.5;
            font-size: 12pt;
        }}
        h1 {{ font-size: 18pt; }}
        h2 {{ font-size: 16pt; }}
        h3 {{ font-size: 14pt; }}
        code, pre {{ font-family: 'Courier New', monospace; }}
    </style>
</head>
<body>
{}
<script>window.print();</script>
</body>
</html>"#,
            html
        );
        
        fs::write(filepath, pdf_html)?;
        Ok(())
    }
    
    /// Export markdown (copy with clean formatting)
    pub fn export_markdown(&self, filepath: &str) -> Result<()> {
        let content = self.get_content()?;
        fs::write(filepath, content)?;
        Ok(())
    }
    
    /// Formatting operations
    pub fn format_bold(&self, selected_text: &str) -> Result<String> {
        self.service.toggle_formatting(selected_text, FormatType::Bold)
    }
    
    pub fn format_italic(&self, selected_text: &str) -> Result<String> {
        self.service.toggle_formatting(selected_text, FormatType::Italic)
    }
    
    pub fn format_strikethrough(&self, selected_text: &str) -> Result<String> {
        self.service.toggle_formatting(selected_text, FormatType::Strikethrough)
    }
    
    pub fn format_code(&self, selected_text: &str) -> Result<String> {
        self.service.toggle_formatting(selected_text, FormatType::Code)
    }
    
    pub fn format_heading(&self, text: &str, level: u8) -> Result<String> {
        self.service.make_heading(text, level)
    }
    
    pub fn format_list(&self, text: &str, list_type: &str) -> Result<String> {
        match list_type {
            "bullet" => self.service.make_unordered_list_item(text),
            "numbered" => self.service.make_ordered_list_item(text),
            "task" => self.service.make_task_list_item(text, false),
            _ => Ok(text.to_string()),
        }
    }
    
    pub fn create_link(&self, text: &str, url: &str) -> Result<String> {
        self.service.create_link(text, url, None)
    }
    
    pub fn create_image(&self, alt: &str, url: &str) -> Result<String> {
        self.service.create_image(alt, url, None)
    }
    
    pub fn create_table(&self, headers: Vec<&str>, rows: Vec<Vec<&str>>) -> Result<String> {
        self.service.create_table(headers, rows)
    }
    
    pub fn format_blockquote(&self, text: &str) -> Result<String> {
        self.service.make_blockquote(text)
    }
    
    /// Insert formatted content at cursor position
    pub fn insert_formatted_content(&self, content: &str, formatted: &str) -> Result<String> {
        let current_content = self.get_content()?;
        let position = self.cursor_position.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock cursor position: {}", e)
        })?;
        
        // For now, just replace the content - in a real implementation,
        // we would use the cursor position to insert at the right location
        let new_content = current_content.replace(content, formatted);
        Ok(new_content)
    }
    
    /// Cursor and selection management
    pub fn update_cursor_position(&self, line: i32, column: i32) -> Result<()> {
        let mut position = self.cursor_position.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock cursor position: {}", e)
        })?;
        *position = (line, column);
        Ok(())
    }
    
    pub fn update_selection(&self, start: i32, length: i32) -> Result<()> {
        let mut selection = self.selection_range.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock selection: {}", e)
        })?;
        *selection = (start, length);
        Ok(())
    }
    
    pub fn get_cursor_position(&self) -> Result<(i32, i32)> {
        let position = self.cursor_position.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock cursor position: {}", e)
        })?;
        Ok(*position)
    }
    
    pub fn get_selection(&self) -> Result<(i32, i32)> {
        let selection = self.selection_range.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock selection: {}", e)
        })?;
        Ok(*selection)
    }
    
    /// Syntax validation and error highlighting
    pub fn validate_syntax(&self, content: &str) -> Result<Vec<ValidationError>> {
        self.service.validate_syntax(content)
    }
    
    /// Get validation errors for UI display
    pub fn get_validation_errors_for_ui(&self, content: &str) -> Result<Vec<UIValidationError>> {
        let errors = self.service.validate_syntax(content)?;
        Ok(errors.into_iter().map(|error| {
            UIValidationError {
                line: error.line as i32,
                column: error.column as i32,
                message: SharedString::from(error.message),
                error_type: SharedString::from(format!("{:?}", error.error_type)),
            }
        }).collect())
    }
    
    /// Get syntax elements for highlighting
    pub fn get_syntax_elements_for_ui(&self, content: &str) -> Result<Vec<UISyntaxElement>> {
        let rendered = self.service.parse_to_elements(content)?;
        let validation_errors = self.service.validate_syntax(content)?;
        
        // Create a set of error positions for quick lookup
        let error_positions: HashSet<(usize, usize)> = validation_errors
            .iter()
            .map(|error| (error.line, error.column))
            .collect();
        
        Ok(rendered.elements.into_iter().map(|element| {
            let has_error = error_positions.contains(&(element.position.start_line, element.position.start_col));
            
            UISyntaxElement {
                element_type: SharedString::from(element.element_type),
                start_line: element.position.start_line as i32,
                start_col: element.position.start_col as i32,
                end_line: element.position.end_line as i32,
                end_col: element.position.end_col as i32,
                valid: !has_error,
            }
        }).collect())
    }
    
    /// Real-time validation with debouncing simulation
    pub fn validate_content_realtime(&self, content: &str) -> Result<(Vec<UIValidationError>, Vec<UISyntaxElement>, i32, i32)> {
        let validation_errors = self.get_validation_errors_for_ui(content)?;
        let syntax_elements = self.get_syntax_elements_for_ui(content)?;
        
        let error_count = validation_errors.iter()
            .filter(|error| matches!(error.error_type.as_str(), "InvalidSyntax" | "UnclosedCodeBlock" | "MalformedTable"))
            .count() as i32;
            
        let warning_count = validation_errors.iter()
            .filter(|error| matches!(error.error_type.as_str(), "MalformedLink" | "MalformedImage" | "InvalidHeading"))
            .count() as i32;
        
        Ok((validation_errors, syntax_elements, error_count, warning_count))
    }
    
    /// Get document state
    pub fn is_modified(&self) -> Result<bool> {
        let modified = self.document_modified.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock modified flag: {}", e)
        })?;
        Ok(*modified)
    }
    
    pub fn get_filename(&self) -> Result<Option<String>> {
        let filename = self.current_filename.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock filename: {}", e)
        })?;
        Ok(filename.clone())
    }
    
    /// Initialize with sample enhanced content
    pub fn create_enhanced_sample_content(&self) -> String {
        r#"# Enhanced Markdown Editor

Welcome to the **Enhanced Markdown Editor** by TradocFlow! This editor provides a comprehensive markdown editing experience with live preview, formatting tools, and advanced features.

## 🚀 Key Features

### Editor Capabilities
- **Split-pane view** with resizable panels
- **Live preview** with inline editing
- **Formatting toolbar** with one-click operations
- **Real-time statistics** and document metrics
- **Syntax highlighting** and error detection
- **Keyboard shortcuts** for power users

### File Operations
- Create, open, and save markdown files
- Export to HTML, PDF, and markdown formats
- Auto-save and modification tracking
- Recent files and project management

## 🎯 Getting Started

### Basic Formatting

Use the toolbar buttons or keyboard shortcuts:

- **Bold text**: `Ctrl+B` or use the **B** button
- *Italic text*: `Ctrl+I` or use the **I** button  
- ~~Strikethrough~~: Use the **S** button
- `Inline code`: Use the **</>** button

### Headings and Structure

Create headings with the H1, H2, H3 buttons:

#### Subheading Example
##### Another Level
###### Final Level

### Lists and Tasks

#### Bullet Lists
- First item
- Second item
  - Nested item
  - Another nested item
- Third item

#### Numbered Lists
1. First step
2. Second step
3. Third step

#### Task Lists
- [x] Completed task
- [x] Another completed task
- [ ] Pending task
- [ ] Future enhancement

### Code Examples

Inline `code snippets` and code blocks:

```javascript
// JavaScript example
function enhancedMarkdownEditor() {
    const features = [
        'live-preview',
        'syntax-highlighting', 
        'formatting-toolbar',
        'export-options'
    ];
    
    return features.map(f => `✓ ${f}`).join('\n');
}
```

```python
# Python example
class MarkdownEditor:
    def __init__(self):
        self.features = {
            'live_preview': True,
            'toolbar': True,
            'statistics': True,
            'export': ['html', 'pdf', 'md']
        }
    
    def render(self, content):
        return self.process_markdown(content)
```

### Links and Images

Visit the [TradocFlow website](https://tradocflow.com) for more information.

![Sample Image](https://via.placeholder.com/600x200/2563eb/ffffff?text=Enhanced+Markdown+Editor)

### Tables

| Feature | Status | Priority | Notes |
|---------|--------|----------|--------|
| Live Preview | ✅ Complete | High | Real-time rendering |
| Formatting Toolbar | ✅ Complete | High | One-click formatting |
| Export Options | ✅ Complete | Medium | HTML, PDF, MD |
| Syntax Highlighting | 🚧 In Progress | Medium | Error detection |
| Plugin System | 📋 Planned | Low | Extensibility |
| Collaborative Editing | 💡 Idea | Low | Real-time collaboration |

### Quotes and Callouts

> **Important Note**: This editor is designed for efficiency and ease of use. The live preview updates automatically as you type, and all formatting operations can be performed with simple toolbar clicks or keyboard shortcuts.

> **Pro Tip**: Use the split-pane view to see your markdown source and rendered output simultaneously. Adjust the split ratio using the toolbar slider.

## 🔧 Advanced Features

### Document Statistics
The status bar shows real-time statistics:
- Word count and character count
- Number of headings, links, and images  
- Table and list counts
- Document modification status

### Keyboard Shortcuts
- `Ctrl+N`: New document
- `Ctrl+O`: Open file
- `Ctrl+S`: Save document
- `Ctrl+Shift+S`: Save as
- `Ctrl+E`: Export options
- `Ctrl+B`: Bold formatting
- `Ctrl+I`: Italic formatting
- `Ctrl+K`: Insert link
- `Ctrl+Shift+I`: Insert image

### Export Options
Export your documents in multiple formats:
- **HTML**: Web-ready format with styling
- **PDF**: Print-ready document format  
- **Markdown**: Clean markdown source

---

**Happy writing!** 📝✨

*Created with TradocFlow Enhanced Markdown Editor - Version 1.0*"#.to_string()
    }
    
    /// Initialize with enhanced sample content
    pub fn initialize_with_enhanced_sample(&self) -> Result<()> {
        let sample_content = self.create_enhanced_sample_content();
        self.update_content(&sample_content)?;
        
        let mut modified = self.document_modified.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock modified flag: {}", e)
        })?;
        *modified = false;
        
        Ok(())
    }
    
    /// Create sample content with intentional validation errors for demonstration
    pub fn create_sample_with_errors(&self) -> String {
        r#"# Sample Document with Validation Examples

This document contains intentional errors to demonstrate validation.

####### Invalid Heading (too many #)

#InvalidHeading (missing space)

[Empty Link]()

![Empty Image]()

```unclosed
This code block is not closed

| Malformed | |

[Unclosed link(

Proper markdown:
- Valid list item
- Another valid item

**Bold text** and *italic text* work fine.

```javascript
function validCode() {
    return "This is properly closed";
}
```

[Valid Link](https://example.com)

![Valid Image](https://via.placeholder.com/200)

| Proper | Table | Structure |
|--------|-------|-----------|  
| Cell 1 | Cell 2| Cell 3    |
"#.to_string()
    }
    
    /// Load sample content with validation errors for demonstration
    pub fn load_sample_with_errors(&self) -> Result<()> {
        let sample_content = self.create_sample_with_errors();
        self.update_content(&sample_content)?;
        
        let mut modified = self.document_modified.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock modified flag: {}", e)
        })?;
        *modified = false;
        
        Ok(())
    }
}

impl Default for EnhancedMarkdownBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_enhanced_bridge_creation() {
        let bridge = EnhancedMarkdownBridge::new();
        assert!(bridge.get_content().is_ok());
        assert!(!bridge.is_modified().unwrap());
    }
    
    #[test]
    fn test_document_statistics() {
        let bridge = EnhancedMarkdownBridge::new();
        let content = "# Title\n\nThis is **bold** text with [link](url) and ![image](url).\n\n- List item";
        let stats = bridge.calculate_statistics(content);
        
        assert!(stats.word_count > 0);
        assert!(stats.character_count > 0);
        assert_eq!(stats.heading_count, 1);
        assert_eq!(stats.link_count, 1);
        assert_eq!(stats.image_count, 1);
    }
    
    #[test]
    fn test_formatting_operations() {
        let bridge = EnhancedMarkdownBridge::new();
        
        let bold = bridge.format_bold("text").unwrap();
        assert_eq!(bold, "**text**");
        
        let italic = bridge.format_italic("text").unwrap();
        assert_eq!(italic, "*text*");
        
        let heading = bridge.format_heading("Title", 2).unwrap();
        assert_eq!(heading, "## Title");
        
        let link = bridge.create_link("Google", "https://google.com").unwrap();
        assert_eq!(link, "[Google](https://google.com)");
    }
    
    #[test]
    fn test_cursor_and_selection() {
        let bridge = EnhancedMarkdownBridge::new();
        
        bridge.update_cursor_position(10, 25).unwrap();
        let pos = bridge.get_cursor_position().unwrap();
        assert_eq!(pos, (10, 25));
        
        bridge.update_selection(5, 15).unwrap();
        let sel = bridge.get_selection().unwrap();
        assert_eq!(sel, (5, 15));
    }
    
    #[test]
    fn test_file_operations() {
        let bridge = EnhancedMarkdownBridge::new();
        
        // Test new document
        bridge.new_document().unwrap();
        assert!(!bridge.is_modified().unwrap());
        assert!(bridge.get_filename().unwrap().is_none());
        
        // Test content update
        bridge.update_content("# Test content").unwrap();
        assert!(bridge.is_modified().unwrap());
        assert_eq!(bridge.get_content().unwrap(), "# Test content");
    }
    
    #[test]
    fn test_enhanced_sample_content() {
        let bridge = EnhancedMarkdownBridge::new();
        bridge.initialize_with_enhanced_sample().unwrap();
        
        let content = bridge.get_content().unwrap();
        assert!(content.contains("Enhanced Markdown Editor"));
        assert!(content.contains("Key Features"));
        assert!(content.contains("```javascript"));
        assert!(content.contains("| Feature |"));
        
        let stats = bridge.calculate_statistics(&content);
        assert!(stats.word_count > 100);
        assert!(stats.heading_count > 5);
    }
}