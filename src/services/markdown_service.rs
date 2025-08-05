use anyhow::Result;
use comrak::{markdown_to_html, ComrakOptions, ComrakExtensionOptions, ComrakParseOptions, ComrakRenderOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownElement {
    pub element_type: String,
    pub content: String,
    pub position: Position,
    pub attributes: HashMap<String, String>,
    pub children: Vec<MarkdownElement>,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedMarkdown {
    pub html: String,
    pub elements: Vec<MarkdownElement>,
    pub metadata: MarkdownMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownMetadata {
    pub word_count: usize,
    pub heading_count: usize,
    pub link_count: usize,
    pub image_count: usize,
    pub table_count: usize,
}

pub struct MarkdownService {
    options: ComrakOptions,
}

impl Default for MarkdownService {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownService {
    pub fn new() -> Self {
        let mut options = ComrakOptions::default();
        
        // Enable GitHub Flavored Markdown extensions
        options.extension.strikethrough = true;
        options.extension.tagfilter = false;
        options.extension.table = true;
        options.extension.autolink = true;
        options.extension.tasklist = true;
        options.extension.superscript = true;
        options.extension.header_ids = Some("heading-".to_string());
        options.extension.footnotes = true;
        options.extension.description_lists = true;
        options.extension.front_matter_delimiter = Some("---".to_string());
        
        // Configure parsing options
        options.parse.smart = true;
        options.parse.default_info_string = Some("text".to_string());
        
        // Configure rendering options
        options.render.hardbreaks = false;
        options.render.github_pre_lang = true;
        options.render.full_info_string = true;
        options.render.width = 0;
        options.render.unsafe_ = false; // Security: disable unsafe HTML
        options.render.escape = false;
        
        Self { options }
    }
    
    /// Render markdown to HTML with live preview enhancements
    pub fn render_to_html(&self, markdown: &str) -> Result<String> {
        let html = markdown_to_html(markdown, &self.options);
        
        // Add interactive classes for live preview
        let enhanced_html = self.add_interactive_classes(&html);
        
        Ok(enhanced_html)
    }
    
    /// Parse markdown and extract editable elements for inline editing
    pub fn parse_to_elements(&self, markdown: &str) -> Result<RenderedMarkdown> {
        let html = self.render_to_html(markdown)?;
        let elements = self.extract_elements(markdown)?;
        let metadata = self.calculate_metadata(markdown);
        
        Ok(RenderedMarkdown {
            html,
            elements,
            metadata,
        })
    }
    
    /// Extract structured elements for inline editing
    fn extract_elements(&self, markdown: &str) -> Result<Vec<MarkdownElement>> {
        let mut elements = Vec::new();
        let lines: Vec<&str> = markdown.lines().collect();
        
        for (line_idx, line) in lines.iter().enumerate() {
            if let Some(element) = self.parse_line(line, line_idx) {
                elements.push(element);
            }
        }
        
        Ok(elements)
    }
    
    /// Parse a single line into a markdown element
    fn parse_line(&self, line: &str, line_idx: usize) -> Option<MarkdownElement> {
        let trimmed = line.trim();
        
        if trimmed.starts_with('#') {
            self.parse_heading(line, line_idx)
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            self.parse_list_item(line, line_idx)
        } else if trimmed.starts_with("1. ") || (trimmed.len() > 2 && trimmed.chars().nth(1) == Some('.')) {
            self.parse_numbered_item(line, line_idx)
        } else if trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]") {
            self.parse_task_item(line, line_idx)
        } else if trimmed.starts_with("> ") {
            self.parse_blockquote(line, line_idx)
        } else if trimmed.starts_with("```") {
            self.parse_code_block(line, line_idx)
        } else if trimmed.starts_with("|") && trimmed.ends_with("|") {
            self.parse_table_row(line, line_idx)
        } else if !trimmed.is_empty() {
            Some(self.parse_paragraph(line, line_idx))
        } else {
            None
        }
    }
    
    fn parse_heading(&self, line: &str, line_idx: usize) -> Option<MarkdownElement> {
        let level = line.chars().take_while(|&c| c == '#').count();
        let content = line.trim_start_matches('#').trim();
        
        Some(MarkdownElement {
            element_type: format!("heading{}", level),
            content: content.to_string(),
            position: Position {
                start_line: line_idx,
                start_col: 0,
                end_line: line_idx,
                end_col: line.len(),
            },
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("level".to_string(), level.to_string());
                attrs.insert("id".to_string(), self.generate_heading_id(content));
                attrs
            },
            children: Vec::new(),
            editable: true,
        })
    }
    
    fn parse_list_item(&self, line: &str, line_idx: usize) -> Option<MarkdownElement> {
        let content = line.trim_start_matches(&['-', '*', '+'][..]).trim();
        
        Some(MarkdownElement {
            element_type: "list_item".to_string(),
            content: content.to_string(),
            position: Position {
                start_line: line_idx,
                start_col: 0,
                end_line: line_idx,
                end_col: line.len(),
            },
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("list_type".to_string(), "unordered".to_string());
                attrs
            },
            children: Vec::new(),
            editable: true,
        })
    }
    
    fn parse_numbered_item(&self, line: &str, line_idx: usize) -> Option<MarkdownElement> {
        let content = line.splitn(2, '.').nth(1).unwrap_or("").trim();
        
        Some(MarkdownElement {
            element_type: "list_item".to_string(),
            content: content.to_string(),
            position: Position {
                start_line: line_idx,
                start_col: 0,
                end_line: line_idx,
                end_col: line.len(),
            },
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("list_type".to_string(), "ordered".to_string());
                attrs
            },
            children: Vec::new(),
            editable: true,
        })
    }
    
    fn parse_task_item(&self, line: &str, line_idx: usize) -> Option<MarkdownElement> {
        let checked = line.contains("[x]");
        let content = if checked {
            line.trim_start_matches("- [x]").trim()
        } else {
            line.trim_start_matches("- [ ]").trim()
        };
        
        Some(MarkdownElement {
            element_type: "task_item".to_string(),
            content: content.to_string(),
            position: Position {
                start_line: line_idx,
                start_col: 0,
                end_line: line_idx,
                end_col: line.len(),
            },
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("checked".to_string(), checked.to_string());
                attrs
            },
            children: Vec::new(),
            editable: true,
        })
    }
    
    fn parse_blockquote(&self, line: &str, line_idx: usize) -> Option<MarkdownElement> {
        let content = line.trim_start_matches('>').trim();
        
        Some(MarkdownElement {
            element_type: "blockquote".to_string(),
            content: content.to_string(),
            position: Position {
                start_line: line_idx,
                start_col: 0,
                end_line: line_idx,
                end_col: line.len(),
            },
            attributes: HashMap::new(),
            children: Vec::new(),
            editable: true,
        })
    }
    
    fn parse_code_block(&self, line: &str, line_idx: usize) -> Option<MarkdownElement> {
        let language = line.trim_start_matches("```").trim();
        
        Some(MarkdownElement {
            element_type: "code_block".to_string(),
            content: String::new(), // Code content would be on following lines
            position: Position {
                start_line: line_idx,
                start_col: 0,
                end_line: line_idx,
                end_col: line.len(),
            },
            attributes: {
                let mut attrs = HashMap::new();
                if !language.is_empty() {
                    attrs.insert("language".to_string(), language.to_string());
                }
                attrs
            },
            children: Vec::new(),
            editable: true,
        })
    }
    
    fn parse_table_row(&self, line: &str, line_idx: usize) -> Option<MarkdownElement> {
        let cells: Vec<String> = line
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect();
        
        Some(MarkdownElement {
            element_type: "table_row".to_string(),
            content: line.to_string(),
            position: Position {
                start_line: line_idx,
                start_col: 0,
                end_line: line_idx,
                end_col: line.len(),
            },
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("cell_count".to_string(), cells.len().to_string());
                attrs
            },
            children: cells.into_iter().enumerate().map(|(i, cell)| {
                MarkdownElement {
                    element_type: "table_cell".to_string(),
                    content: cell,
                    position: Position {
                        start_line: line_idx,
                        start_col: 0, // Would need more precise calculation
                        end_line: line_idx,
                        end_col: 0,
                    },
                    attributes: {
                        let mut attrs = HashMap::new();
                        attrs.insert("column".to_string(), i.to_string());
                        attrs
                    },
                    children: Vec::new(),
                    editable: true,
                }
            }).collect(),
            editable: false, // Row itself not editable, but cells are
        })
    }
    
    fn parse_paragraph(&self, line: &str, line_idx: usize) -> MarkdownElement {
        MarkdownElement {
            element_type: "paragraph".to_string(),
            content: line.to_string(),
            position: Position {
                start_line: line_idx,
                start_col: 0,
                end_line: line_idx,
                end_col: line.len(),
            },
            attributes: HashMap::new(),
            children: Vec::new(),
            editable: true,
        }
    }
    
    /// Calculate metadata for the markdown document
    fn calculate_metadata(&self, markdown: &str) -> MarkdownMetadata {
        let word_count = markdown
            .split_whitespace()
            .count();
        
        let heading_count = markdown
            .lines()
            .filter(|line| line.trim_start().starts_with('#'))
            .count();
        
        let link_count = markdown
            .matches("[")
            .count()
            .min(markdown.matches("](").count());
        
        let image_count = markdown
            .matches("![")
            .count();
        
        let table_count = markdown
            .lines()
            .filter(|line| line.trim_start().starts_with('|') && line.trim_end().ends_with('|'))
            .count();
        
        MarkdownMetadata {
            word_count,
            heading_count,
            link_count,
            image_count,
            table_count,
        }
    }
    
    /// Add interactive CSS classes for live preview functionality
    fn add_interactive_classes(&self, html: &str) -> String {
        html
            .replace("<h1", "<h1 class=\"editable-heading editable-element\" data-type=\"heading1\"")
            .replace("<h2", "<h2 class=\"editable-heading editable-element\" data-type=\"heading2\"")
            .replace("<h3", "<h3 class=\"editable-heading editable-element\" data-type=\"heading3\"")
            .replace("<h4", "<h4 class=\"editable-heading editable-element\" data-type=\"heading4\"")
            .replace("<h5", "<h5 class=\"editable-heading editable-element\" data-type=\"heading5\"")
            .replace("<h6", "<h6 class=\"editable-heading editable-element\" data-type=\"heading6\"")
            .replace("<p", "<p class=\"editable-paragraph editable-element\" data-type=\"paragraph\"")
            .replace("<li", "<li class=\"editable-list-item editable-element\" data-type=\"list_item\"")
            .replace("<blockquote", "<blockquote class=\"editable-blockquote editable-element\" data-type=\"blockquote\"")
            .replace("<pre><code", "<pre><code class=\"editable-code-block editable-element\" data-type=\"code_block\"")
            .replace("<td", "<td class=\"editable-table-cell editable-element\" data-type=\"table_cell\"")
            .replace("<th", "<th class=\"editable-table-header editable-element\" data-type=\"table_header\"")
    }
    
    /// Generate a heading ID from content
    fn generate_heading_id(&self, content: &str) -> String {
        content
            .to_lowercase()
            .chars()
            .filter_map(|c| {
                if c.is_alphanumeric() {
                    Some(c)
                } else if c.is_whitespace() {
                    Some('-')
                } else {
                    None
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }
    
    /// Update markdown content with inline edits
    pub fn update_element(&self, markdown: &str, element_id: &str, new_content: &str) -> Result<String> {
        // This would implement the logic to update specific elements
        // For now, returning the original markdown
        // In a full implementation, this would:
        // 1. Parse the markdown to find the element
        // 2. Replace the element's content
        // 3. Return the updated markdown
        
        Ok(markdown.to_string())
    }
    
    /// Convert HTML back to markdown (for inline editing)
    pub fn html_to_markdown(&self, html: &str) -> Result<String> {
        // This is a simplified conversion
        // In practice, you'd want a proper HTML to Markdown converter
        let markdown = html
            .replace("<h1>", "# ")
            .replace("</h1>", "\n")
            .replace("<h2>", "## ")
            .replace("</h2>", "\n")
            .replace("<h3>", "### ")
            .replace("</h3>", "\n")
            .replace("<p>", "")
            .replace("</p>", "\n\n")
            .replace("<strong>", "**")
            .replace("</strong>", "**")
            .replace("<em>", "*")
            .replace("</em>", "*")
            .replace("<li>", "- ")
            .replace("</li>", "\n")
            .replace("<ul>", "")
            .replace("</ul>", "")
            .replace("<ol>", "")
            .replace("</ol>", "")
            .replace("<blockquote>", "> ")
            .replace("</blockquote>", "\n");
        
        Ok(markdown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_markdown_rendering() {
        let service = MarkdownService::new();
        let markdown = "# Hello World\n\nThis is a **bold** text.";
        let result = service.render_to_html(markdown).unwrap();
        
        assert!(result.contains("<h1"));
        assert!(result.contains("Hello World"));
        assert!(result.contains("<strong>"));
        assert!(result.contains("bold"));
    }
    
    #[test]
    fn test_element_extraction() {
        let service = MarkdownService::new();
        let markdown = "# Heading\n\nParagraph text\n\n- List item";
        let result = service.parse_to_elements(markdown).unwrap();
        
        assert_eq!(result.elements.len(), 3);
        assert_eq!(result.elements[0].element_type, "heading1");
        assert_eq!(result.elements[1].element_type, "paragraph");
        assert_eq!(result.elements[2].element_type, "list_item");
    }
    
    #[test]
    fn test_metadata_calculation() {
        let service = MarkdownService::new();
        let markdown = "# Heading\n\n[Link](url) and ![Image](url)\n\n| Table | Cell |\n|-------|------|";
        let result = service.parse_to_elements(markdown).unwrap();
        
        assert_eq!(result.metadata.heading_count, 1);
        assert_eq!(result.metadata.link_count, 1);
        assert_eq!(result.metadata.image_count, 1);
        assert!(result.metadata.word_count > 0);
    }
}