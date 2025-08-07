use crate::services::markdown_service::{MarkdownService, RenderedMarkdown, MarkdownElement as ServiceMarkdownElement};
use anyhow::Result;
use slint::{ModelRc, SharedString};
use std::collections::HashMap;

// Simplified Slint types for demo purposes
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

/// Bridge between Rust markdown services and Slint UI components
pub struct MarkdownEditorBridge {
    service: MarkdownService,
    current_content: std::sync::Mutex<String>,
}

impl MarkdownEditorBridge {
    /// Create a new markdown editor bridge
    pub fn new() -> Self {
        Self {
            service: MarkdownService::new(),
            current_content: std::sync::Mutex::new(String::new()),
        }
    }
    
    /// Convert service markdown content to Slint rendered content
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
    
    /// Update markdown content and trigger re-render
    pub fn update_content(&self, content: &str) -> Result<()> {
        let mut current_content = self.current_content.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock content: {}", e)
        })?;
        *current_content = content.to_string();
        Ok(())
    }
    
    /// Get current markdown content
    pub fn get_content(&self) -> Result<String> {
        let current_content = self.current_content.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock content: {}", e)
        })?;
        Ok(current_content.clone())
    }
    
    /// Handle element editing from the UI
    pub fn handle_element_edit(&self, element_id: &str, new_content: &str) -> Result<String> {
        let current_content = self.get_content()?;
        let updated_content = self.update_markdown_element(&current_content, element_id, new_content)?;
        self.update_content(&updated_content)?;
        Ok(updated_content)
    }
    
    /// Update a specific markdown element
    fn update_markdown_element(&self, markdown: &str, element_id: &str, new_content: &str) -> Result<String> {
        // Parse element ID to get type and line number
        let parts: Vec<&str> = element_id.split('-').collect();
        if parts.len() < 2 {
            return Ok(markdown.to_string());
        }
        
        let element_type = parts[0];
        let line_number: usize = parts[1].parse().unwrap_or(0);
        
        let mut lines: Vec<String> = markdown.lines().map(String::from).collect();
        
        if line_number >= lines.len() {
            return Ok(markdown.to_string());
        }
        
        // Update the specific line based on element type
        match element_type {
            "heading1" => lines[line_number] = format!("# {}", new_content),
            "heading2" => lines[line_number] = format!("## {}", new_content),
            "heading3" => lines[line_number] = format!("### {}", new_content),
            "heading4" => lines[line_number] = format!("#### {}", new_content),
            "heading5" => lines[line_number] = format!("##### {}", new_content),
            "heading6" => lines[line_number] = format!("###### {}", new_content),
            "paragraph" => lines[line_number] = new_content.to_string(),
            "list_item" => {
                // Preserve list marker
                if lines[line_number].trim_start().starts_with("- ") {
                    lines[line_number] = format!("- {}", new_content);
                } else if lines[line_number].trim_start().starts_with("* ") {
                    lines[line_number] = format!("* {}", new_content);
                } else if lines[line_number].trim_start().starts_with("+ ") {
                    lines[line_number] = format!("+ {}", new_content);
                } else {
                    // Find list number if it's an ordered list
                    if let Some(dot_pos) = lines[line_number].find('.') {
                        let number_part = &lines[line_number][..dot_pos + 1];
                        lines[line_number] = format!("{} {}", number_part, new_content);
                    } else {
                        lines[line_number] = format!("- {}", new_content);
                    }
                }
            }
            "task_item" => {
                if lines[line_number].contains("[x]") {
                    lines[line_number] = format!("- [x] {}", new_content);
                } else {
                    lines[line_number] = format!("- [ ] {}", new_content);
                }
            }
            "blockquote" => lines[line_number] = format!("> {}", new_content),
            _ => lines[line_number] = new_content.to_string(),
        }
        
        Ok(lines.join("\n"))
    }
    
    /// Export rendered HTML
    pub fn export_html(&self) -> Result<String> {
        let content = self.get_content()?;
        self.service.render_to_html(&content)
    }
    
    /// Get markdown statistics
    pub fn get_statistics(&self) -> Result<HashMap<String, i32>> {
        let content = self.get_content()?;
        let rendered = self.service.parse_to_elements(&content)?;
        
        let mut stats = HashMap::new();
        stats.insert("words".to_string(), rendered.metadata.word_count as i32);
        stats.insert("headings".to_string(), rendered.metadata.heading_count as i32);
        stats.insert("links".to_string(), rendered.metadata.link_count as i32);
        stats.insert("images".to_string(), rendered.metadata.image_count as i32);
        stats.insert("tables".to_string(), rendered.metadata.table_count as i32);
        stats.insert("elements".to_string(), rendered.elements.len() as i32);
        
        Ok(stats)
    }
    
    /// Create sample markdown content for testing
    pub fn create_sample_content(&self) -> String {
        r#"# Sample Document

This is a **sample document** to demonstrate the live preview functionality of the markdown editor.

## Features

- **Live Preview**: See your markdown rendered in real-time
- **Inline Editing**: Click on any element to edit it directly
- **Syntax Highlighting**: Full support for markdown syntax
- **Word Count**: Track your document statistics

### Code Example

```rust
fn main() {
    println!("Hello, markdown world!");
}
```

### Task List

- [x] Implement basic markdown rendering
- [x] Add live preview functionality
- [ ] Add inline editing capabilities
- [ ] Implement syntax highlighting

> **Note**: This is a blockquote to show how different elements are rendered.

## Tables

| Feature | Status | Priority |
|---------|--------|----------|
| Live Preview | ✅ Complete | High |
| Inline Editing | 🚧 In Progress | High |
| Syntax Highlighting | 📋 Planned | Medium |

## Links and Images

Check out the [markdown guide](https://commonmark.org/) for more information.

![Sample Image](https://via.placeholder.com/300x200?text=Sample+Image)

---

*This document was created with the TradocFlow markdown editor.*"#.to_string()
    }
    
    /// Initialize with sample content
    pub fn initialize_with_sample(&self) -> Result<()> {
        let sample_content = self.create_sample_content();
        self.update_content(&sample_content)?;
        Ok(())
    }
    
    /// Convert HTML back to markdown (for clipboard operations)
    pub fn html_to_markdown(&self, html: &str) -> Result<String> {
        self.service.html_to_markdown(html)
    }
    
    /// Validate markdown syntax
    pub fn validate_markdown(&self, markdown: &str) -> Result<Vec<String>> {
        // Basic markdown validation
        let mut warnings = Vec::new();
        
        // Check for common markdown issues
        let lines: Vec<&str> = markdown.lines().collect();
        
        for (idx, line) in lines.iter().enumerate() {
            // Check for unmatched brackets
            let open_brackets = line.matches('[').count();
            let close_brackets = line.matches(']').count();
            if open_brackets != close_brackets {
                warnings.push(format!("Line {}: Unmatched brackets", idx + 1));
            }
            
            // Check for unmatched parentheses in links
            let open_parens = line.matches('(').count();
            let close_parens = line.matches(')').count();
            if open_parens != close_parens && (line.contains("](") || line.contains("![")) {
                warnings.push(format!("Line {}: Possible malformed link", idx + 1));
            }
            
            // Check for heading levels
            if line.starts_with('#') {
                let level = line.chars().take_while(|&c| c == '#').count();
                if level > 6 {
                    warnings.push(format!("Line {}: Heading level too deep (max 6)", idx + 1));
                }
            }
        }
        
        Ok(warnings)
    }
}

impl Default for MarkdownEditorBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_markdown_bridge_creation() {
        let bridge = MarkdownEditorBridge::new();
        assert!(bridge.get_content().is_ok());
    }
    
    #[test]
    fn test_content_update() {
        let bridge = MarkdownEditorBridge::new();
        let test_content = "# Test Heading\n\nTest content";
        
        assert!(bridge.update_content(test_content).is_ok());
        assert_eq!(bridge.get_content().unwrap(), test_content);
    }
    
    #[test]
    fn test_element_editing() {
        let bridge = MarkdownEditorBridge::new();
        let initial_content = "# Old Heading\n\nSome content";
        bridge.update_content(initial_content).unwrap();
        
        let updated = bridge.handle_element_edit("heading1-0", "New Heading").unwrap();
        assert!(updated.contains("# New Heading"));
    }
    
    #[test]
    fn test_markdown_validation() {
        let bridge = MarkdownEditorBridge::new();
        let invalid_markdown = "# Heading\n\n[Unmatched bracket\n\n####### Too deep heading";
        
        let warnings = bridge.validate_markdown(invalid_markdown).unwrap();
        assert!(warnings.len() > 0);
        assert!(warnings.iter().any(|w| w.contains("Unmatched brackets")));
        assert!(warnings.iter().any(|w| w.contains("too deep")));
    }
    
    #[test]
    fn test_sample_content() {
        let bridge = MarkdownEditorBridge::new();
        let sample = bridge.create_sample_content();
        
        assert!(sample.contains("# Sample Document"));
        assert!(sample.contains("- [x]"));
        assert!(sample.contains("```rust"));
        assert!(sample.contains("| Feature |"));
    }
}