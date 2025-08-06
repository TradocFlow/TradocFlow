use anyhow::Result;
use comrak::{markdown_to_html, ComrakOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub error_type: ValidationErrorType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorType {
    InvalidSyntax,
    MalformedLink,
    MalformedImage,
    MalformedTable,
    UnclosedCodeBlock,
    InvalidHeading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormatType {
    Bold,
    Italic,
    Strikethrough,
    Code,
    Link { url: String, title: Option<String> },
    Image { url: String, alt: String, title: Option<String> },
    Heading { level: u8 },
    UnorderedList,
    OrderedList,
    TaskList { checked: bool },
    Blockquote,
    CodeBlock { language: Option<String> },
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
}

pub struct MarkdownService {
    options: ComrakOptions<'static>,
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
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i];
            
            // Handle multi-line elements like code blocks
            if line.trim().starts_with("```") {
                if let Some((code_element, lines_consumed)) = self.parse_code_block(&lines, i) {
                    elements.push(code_element);
                    i += lines_consumed;
                    continue;
                }
            }
            
            // Handle multi-line blockquotes
            if line.trim().starts_with("> ") {
                if let Some((quote_element, lines_consumed)) = self.parse_multi_line_blockquote(&lines, i) {
                    elements.push(quote_element);
                    i += lines_consumed;
                    continue;
                }
            }
            
            // Handle list items with potential nesting
            if self.is_list_item(line) {
                if let Some((list_element, lines_consumed)) = self.parse_list_structure(&lines, i) {
                    elements.push(list_element);
                    i += lines_consumed;
                    continue;
                }
            }
            
            // Handle table structures
            if self.is_table_row(line) {
                if let Some((table_element, lines_consumed)) = self.parse_table_structure(&lines, i) {
                    elements.push(table_element);
                    i += lines_consumed;
                    continue;
                }
            }
            
            // Handle single-line elements
            if let Some(element) = self.parse_line(line, i) {
                elements.push(element);
            }
            
            i += 1;
        }
        
        Ok(elements)
    }
    
    /// Parse multi-line code block
    fn parse_code_block(&self, lines: &[&str], start_idx: usize) -> Option<(MarkdownElement, usize)> {
        let start_line = lines[start_idx].trim();
        if !start_line.starts_with("```") {
            return None;
        }
        
        let language = start_line.trim_start_matches("```").trim();
        let mut content = String::new();
        let mut end_idx = start_idx + 1;
        
        // Find the closing ```
        while end_idx < lines.len() {
            let line = lines[end_idx];
            if line.trim() == "```" {
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            end_idx += 1;
        }
        
        let element = MarkdownElement {
            element_type: "code_block".to_string(),
            content,
            position: Position {
                start_line: start_idx,
                start_col: 0,
                end_line: end_idx,
                end_col: if end_idx < lines.len() { lines[end_idx].len() } else { 0 },
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
        };
        
        Some((element, end_idx - start_idx + 1))
    }
    
    /// Parse multi-line blockquote
    fn parse_multi_line_blockquote(&self, lines: &[&str], start_idx: usize) -> Option<(MarkdownElement, usize)> {
        let mut content = String::new();
        let mut end_idx = start_idx;
        
        while end_idx < lines.len() {
            let line = lines[end_idx];
            if !line.trim().starts_with("> ") && !line.trim().is_empty() {
                break;
            }
            
            if line.trim().starts_with("> ") {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(line.trim_start_matches('>').trim());
            }
            end_idx += 1;
        }
        
        if content.is_empty() {
            return None;
        }
        
        let element = MarkdownElement {
            element_type: "blockquote".to_string(),
            content,
            position: Position {
                start_line: start_idx,
                start_col: 0,
                end_line: end_idx - 1,
                end_col: if end_idx > 0 { lines[end_idx - 1].len() } else { 0 },
            },
            attributes: HashMap::new(),
            children: Vec::new(),
            editable: true,
        };
        
        Some((element, end_idx - start_idx))
    }
    
    /// Check if line is a list item
    fn is_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("- ") || 
        trimmed.starts_with("* ") || 
        trimmed.starts_with("+ ") ||
        trimmed.starts_with("- [") ||
        (trimmed.len() > 2 && trimmed.chars().nth(1) == Some('.') && trimmed.chars().nth(0).unwrap().is_ascii_digit())
    }
    
    /// Parse list structure with nesting
    fn parse_list_structure(&self, lines: &[&str], start_idx: usize) -> Option<(MarkdownElement, usize)> {
        let mut list_items = Vec::new();
        let mut end_idx = start_idx;
        let base_indent = self.get_line_indent(lines[start_idx]);
        
        while end_idx < lines.len() {
            let line = lines[end_idx];
            let line_indent = self.get_line_indent(line);
            
            // Stop if we hit a non-list line at the same or lower indentation
            if !self.is_list_item(line) && line_indent <= base_indent && !line.trim().is_empty() {
                break;
            }
            
            if self.is_list_item(line) && line_indent >= base_indent {
                if let Some(item) = self.parse_line(line, end_idx) {
                    list_items.push(item);
                }
            }
            
            end_idx += 1;
        }
        
        if list_items.is_empty() {
            return None;
        }
        
        let list_type = if lines[start_idx].trim().starts_with("- [") {
            "task_list"
        } else if lines[start_idx].trim().chars().nth(0).unwrap().is_ascii_digit() {
            "ordered_list"
        } else {
            "unordered_list"
        };
        
        let element = MarkdownElement {
            element_type: list_type.to_string(),
            content: String::new(),
            position: Position {
                start_line: start_idx,
                start_col: 0,
                end_line: end_idx - 1,
                end_col: if end_idx > 0 { lines[end_idx - 1].len() } else { 0 },
            },
            attributes: HashMap::new(),
            children: list_items,
            editable: false, // List container itself is not editable, but items are
        };
        
        Some((element, end_idx - start_idx))
    }
    
    /// Check if line is a table row
    fn is_table_row(&self, line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 2
    }
    
    /// Parse table structure
    fn parse_table_structure(&self, lines: &[&str], start_idx: usize) -> Option<(MarkdownElement, usize)> {
        let mut table_rows = Vec::new();
        let mut end_idx = start_idx;
        
        while end_idx < lines.len() {
            let line = lines[end_idx];
            if !self.is_table_row(line) {
                break;
            }
            
            // Skip separator rows (|---|---|)
            if line.trim().chars().all(|c| c == '|' || c == '-' || c.is_whitespace()) {
                end_idx += 1;
                continue;
            }
            
            if let Some(row) = self.parse_line(line, end_idx) {
                table_rows.push(row);
            }
            
            end_idx += 1;
        }
        
        if table_rows.is_empty() {
            return None;
        }
        
        let element = MarkdownElement {
            element_type: "table".to_string(),
            content: String::new(),
            position: Position {
                start_line: start_idx,
                start_col: 0,
                end_line: end_idx - 1,
                end_col: if end_idx > 0 { lines[end_idx - 1].len() } else { 0 },
            },
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("row_count".to_string(), table_rows.len().to_string());
                attrs
            },
            children: table_rows,
            editable: false, // Table container itself is not editable, but cells are
        };
        
        Some((element, end_idx - start_idx))
    }
    
    /// Get indentation level of a line
    fn get_line_indent(&self, line: &str) -> usize {
        line.chars().take_while(|c| c.is_whitespace()).count()
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
            self.parse_code_block_start(line, line_idx)
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
    
    fn parse_code_block_start(&self, line: &str, line_idx: usize) -> Option<MarkdownElement> {
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
    pub fn update_element(&self, markdown: &str, element_position: &Position, new_content: &str) -> Result<String> {
        let lines: Vec<&str> = markdown.lines().collect();
        let mut updated_lines: Vec<String> = lines.iter().map(|&s| s.to_string()).collect();
        
        // Handle single-line updates
        if element_position.start_line == element_position.end_line {
            if element_position.start_line < updated_lines.len() {
                let line = &updated_lines[element_position.start_line];
                let before = &line[..element_position.start_col.min(line.len())];
                let after = &line[element_position.end_col.min(line.len())..];
                let updated_line = format!("{}{}{}", before, new_content, after);
                updated_lines[element_position.start_line] = updated_line;
            }
        } else {
            // Handle multi-line updates
            if element_position.start_line < updated_lines.len() {
                // Replace the range with new content
                let new_content_lines: Vec<String> = new_content.lines().map(|s| s.to_string()).collect();
                
                // Remove old lines
                for _ in element_position.start_line..=element_position.end_line.min(updated_lines.len() - 1) {
                    if element_position.start_line < updated_lines.len() {
                        updated_lines.remove(element_position.start_line);
                    }
                }
                
                // Insert new lines
                for (i, new_line) in new_content_lines.iter().enumerate() {
                    updated_lines.insert(element_position.start_line + i, new_line.clone());
                }
            }
        }
        
        Ok(updated_lines.join("\n"))
    }
    
    /// Insert new element at specified position
    pub fn insert_element(&self, markdown: &str, line_number: usize, element_content: &str) -> Result<String> {
        let mut lines: Vec<String> = markdown.lines().map(|s| s.to_string()).collect();
        
        if line_number <= lines.len() {
            lines.insert(line_number, element_content.to_string());
        } else {
            lines.push(element_content.to_string());
        }
        
        Ok(lines.join("\n"))
    }
    
    /// Delete element at specified position
    pub fn delete_element(&self, markdown: &str, element_position: &Position) -> Result<String> {
        let lines: Vec<&str> = markdown.lines().collect();
        let mut updated_lines = Vec::new();
        
        for (i, &line) in lines.iter().enumerate() {
            if i < element_position.start_line || i > element_position.end_line {
                updated_lines.push(line);
            }
        }
        
        Ok(updated_lines.join("\n"))
    }
    
    /// Move element from one position to another
    pub fn move_element(&self, markdown: &str, from_position: &Position, to_line: usize) -> Result<String> {
        // First extract the element content
        let lines: Vec<&str> = markdown.lines().collect();
        let mut element_lines = Vec::new();
        
        for i in from_position.start_line..=from_position.end_line.min(lines.len() - 1) {
            if i < lines.len() {
                element_lines.push(lines[i].to_string());
            }
        }
        
        // Delete the element from original position
        let without_element = self.delete_element(markdown, from_position)?;
        
        // Insert at new position
        let element_content = element_lines.join("\n");
        self.insert_element(&without_element, to_line, &element_content)
    }
    
    /// Find elements by type
    pub fn find_elements_by_type(&self, markdown: &str, element_type: &str) -> Result<Vec<MarkdownElement>> {
        let rendered = self.parse_to_elements(markdown)?;
        let mut matching_elements = Vec::new();
        
        self.collect_elements_by_type(&rendered.elements, element_type, &mut matching_elements);
        
        Ok(matching_elements)
    }
    
    /// Recursively collect elements by type
    fn collect_elements_by_type(&self, elements: &[MarkdownElement], target_type: &str, result: &mut Vec<MarkdownElement>) {
        for element in elements {
            if element.element_type == target_type {
                result.push(element.clone());
            }
            self.collect_elements_by_type(&element.children, target_type, result);
        }
    }
    
    /// Get element at specific position
    pub fn get_element_at_position(&self, markdown: &str, line: usize, column: usize) -> Result<Option<MarkdownElement>> {
        let rendered = self.parse_to_elements(markdown)?;
        
        for element in &rendered.elements {
            if self.position_contains(&element.position, line, column) {
                return Ok(Some(element.clone()));
            }
            
            // Check children
            if let Some(child) = self.find_element_at_position_in_children(&element.children, line, column) {
                return Ok(Some(child));
            }
        }
        
        Ok(None)
    }
    
    /// Check if position contains the given line and column
    fn position_contains(&self, position: &Position, line: usize, column: usize) -> bool {
        if line < position.start_line || line > position.end_line {
            return false;
        }
        
        if line == position.start_line && column < position.start_col {
            return false;
        }
        
        if line == position.end_line && column > position.end_col {
            return false;
        }
        
        true
    }
    
    /// Find element at position in children
    fn find_element_at_position_in_children(&self, children: &[MarkdownElement], line: usize, column: usize) -> Option<MarkdownElement> {
        for child in children {
            if self.position_contains(&child.position, line, column) {
                return Some(child.clone());
            }
            
            if let Some(grandchild) = self.find_element_at_position_in_children(&child.children, line, column) {
                return Some(grandchild);
            }
        }
        
        None
    }
    
    /// Apply formatting to text programmatically
    pub fn apply_formatting(&self, text: &str, format: FormatType) -> Result<String> {
        match format {
            FormatType::Bold => Ok(format!("**{}**", text)),
            FormatType::Italic => Ok(format!("*{}*", text)),
            FormatType::Strikethrough => Ok(format!("~~{}~~", text)),
            FormatType::Code => Ok(format!("`{}`", text)),
            FormatType::Link { url, title } => {
                if let Some(title) = title {
                    Ok(format!("[{}]({} \"{}\")", text, url, title))
                } else {
                    Ok(format!("[{}]({})", text, url))
                }
            },
            FormatType::Image { url, alt, title } => {
                if let Some(title) = title {
                    Ok(format!("![{}]({} \"{}\")", alt, url, title))
                } else {
                    Ok(format!("![{}]({})", alt, url))
                }
            },
            FormatType::Heading { level } => {
                let hashes = "#".repeat(level as usize);
                Ok(format!("{} {}", hashes, text))
            },
            FormatType::UnorderedList => Ok(format!("- {}", text)),
            FormatType::OrderedList => Ok(format!("1. {}", text)),
            FormatType::TaskList { checked } => {
                if checked {
                    Ok(format!("- [x] {}", text))
                } else {
                    Ok(format!("- [ ] {}", text))
                }
            },
            FormatType::Blockquote => Ok(format!("> {}", text)),
            FormatType::CodeBlock { language } => {
                if let Some(lang) = language {
                    Ok(format!("```{}\n{}\n```", lang, text))
                } else {
                    Ok(format!("```\n{}\n```", text))
                }
            },
            FormatType::Table { headers, rows } => {
                let mut table = String::new();
                
                // Add headers
                table.push_str("| ");
                table.push_str(&headers.join(" | "));
                table.push_str(" |\n");
                
                // Add separator
                table.push_str("|");
                for _ in &headers {
                    table.push_str("-------|");
                }
                table.push('\n');
                
                // Add rows
                for row in rows {
                    table.push_str("| ");
                    table.push_str(&row.join(" | "));
                    table.push_str(" |\n");
                }
                
                Ok(table)
            },
        }
    }
    
    /// Validate markdown syntax and return any errors found
    pub fn validate_syntax(&self, markdown: &str) -> Result<Vec<ValidationError>> {
        let mut errors = Vec::new();
        let lines: Vec<&str> = markdown.lines().collect();
        
        // Track code block state
        let mut in_code_block = false;
        let mut code_block_start_line = 0;
        
        for (line_idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            
            // Check for code block markers
            if trimmed.starts_with("```") {
                if in_code_block {
                    in_code_block = false;
                } else {
                    in_code_block = true;
                    code_block_start_line = line_idx;
                }
                continue;
            }
            
            // Skip validation inside code blocks
            if in_code_block {
                continue;
            }
            
            // Validate headings
            if let Some(error) = self.validate_heading(line, line_idx) {
                errors.push(error);
            }
            
            // Validate links
            errors.extend(self.validate_links(line, line_idx));
            
            // Validate images
            errors.extend(self.validate_images(line, line_idx));
            
            // Validate tables
            if let Some(error) = self.validate_table_row(line, line_idx) {
                errors.push(error);
            }
        }
        
        // Check for unclosed code blocks
        if in_code_block {
            errors.push(ValidationError {
                line: code_block_start_line,
                column: 0,
                message: "Unclosed code block".to_string(),
                error_type: ValidationErrorType::UnclosedCodeBlock,
            });
        }
        
        Ok(errors)
    }
    
    fn validate_heading(&self, line: &str, line_idx: usize) -> Option<ValidationError> {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            return None;
        }
        
        let level = trimmed.chars().take_while(|&c| c == '#').count();
        
        // Check for valid heading levels (1-6)
        if level > 6 {
            return Some(ValidationError {
                line: line_idx,
                column: 0,
                message: "Heading level cannot exceed 6".to_string(),
                error_type: ValidationErrorType::InvalidHeading,
            });
        }
        
        // Check for space after hashes
        if level < trimmed.len() && !trimmed.chars().nth(level).unwrap().is_whitespace() {
            return Some(ValidationError {
                line: line_idx,
                column: level,
                message: "Heading must have space after #".to_string(),
                error_type: ValidationErrorType::InvalidHeading,
            });
        }
        
        None
    }
    
    fn validate_links(&self, line: &str, line_idx: usize) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let link_regex = Regex::new(r"\[([^\]]*)\]\(([^)]*)\)").unwrap();
        
        for cap in link_regex.captures_iter(line) {
            let full_match = cap.get(0).unwrap();
            let url = cap.get(2).unwrap().as_str();
            
            // Basic URL validation
            if url.is_empty() {
                errors.push(ValidationError {
                    line: line_idx,
                    column: full_match.start(),
                    message: "Link URL cannot be empty".to_string(),
                    error_type: ValidationErrorType::MalformedLink,
                });
            }
        }
        
        // Check for malformed links (missing closing brackets/parentheses)
        let open_brackets = line.matches('[').count();
        let close_brackets = line.matches(']').count();
        let open_parens = line.matches('(').count();
        let close_parens = line.matches(')').count();
        
        if open_brackets != close_brackets || open_parens != close_parens {
            errors.push(ValidationError {
                line: line_idx,
                column: 0,
                message: "Malformed link syntax - mismatched brackets or parentheses".to_string(),
                error_type: ValidationErrorType::MalformedLink,
            });
        }
        
        errors
    }
    
    fn validate_images(&self, line: &str, line_idx: usize) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let image_regex = Regex::new(r"!\[([^\]]*)\]\(([^)]*)\)").unwrap();
        
        for cap in image_regex.captures_iter(line) {
            let full_match = cap.get(0).unwrap();
            let url = cap.get(2).unwrap().as_str();
            
            // Basic URL validation
            if url.is_empty() {
                errors.push(ValidationError {
                    line: line_idx,
                    column: full_match.start(),
                    message: "Image URL cannot be empty".to_string(),
                    error_type: ValidationErrorType::MalformedImage,
                });
            }
        }
        
        errors
    }
    
    fn validate_table_row(&self, line: &str, line_idx: usize) -> Option<ValidationError> {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            return None;
        }
        
        // Check for proper table structure
        let cells: Vec<&str> = trimmed
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .collect();
        
        // Tables should have at least one cell
        if cells.is_empty() || (cells.len() == 1 && cells[0].trim().is_empty()) {
            return Some(ValidationError {
                line: line_idx,
                column: 0,
                message: "Table row must contain at least one cell".to_string(),
                error_type: ValidationErrorType::MalformedTable,
            });
        }
        
        None
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
    
    /// Apply bold formatting to selected text
    pub fn make_bold(&self, text: &str) -> Result<String> {
        self.apply_formatting(text, FormatType::Bold)
    }
    
    /// Apply italic formatting to selected text
    pub fn make_italic(&self, text: &str) -> Result<String> {
        self.apply_formatting(text, FormatType::Italic)
    }
    
    /// Apply strikethrough formatting to selected text
    pub fn make_strikethrough(&self, text: &str) -> Result<String> {
        self.apply_formatting(text, FormatType::Strikethrough)
    }
    
    /// Apply inline code formatting to selected text
    pub fn make_inline_code(&self, text: &str) -> Result<String> {
        self.apply_formatting(text, FormatType::Code)
    }
    
    /// Convert text to heading with specified level
    pub fn make_heading(&self, text: &str, level: u8) -> Result<String> {
        if level < 1 || level > 6 {
            return Err(anyhow::anyhow!("Heading level must be between 1 and 6"));
        }
        self.apply_formatting(text, FormatType::Heading { level })
    }
    
    /// Create a link with the given text and URL
    pub fn create_link(&self, text: &str, url: &str, title: Option<&str>) -> Result<String> {
        self.apply_formatting(text, FormatType::Link {
            url: url.to_string(),
            title: title.map(|t| t.to_string()),
        })
    }
    
    /// Create an image with the given alt text and URL
    pub fn create_image(&self, alt: &str, url: &str, title: Option<&str>) -> Result<String> {
        self.apply_formatting("", FormatType::Image {
            url: url.to_string(),
            alt: alt.to_string(),
            title: title.map(|t| t.to_string()),
        })
    }
    
    /// Convert text to unordered list item
    pub fn make_unordered_list_item(&self, text: &str) -> Result<String> {
        self.apply_formatting(text, FormatType::UnorderedList)
    }
    
    /// Convert text to ordered list item
    pub fn make_ordered_list_item(&self, text: &str) -> Result<String> {
        self.apply_formatting(text, FormatType::OrderedList)
    }
    
    /// Convert text to task list item
    pub fn make_task_list_item(&self, text: &str, checked: bool) -> Result<String> {
        self.apply_formatting(text, FormatType::TaskList { checked })
    }
    
    /// Convert text to blockquote
    pub fn make_blockquote(&self, text: &str) -> Result<String> {
        self.apply_formatting(text, FormatType::Blockquote)
    }
    
    /// Create a code block with optional language
    pub fn create_code_block(&self, code: &str, language: Option<&str>) -> Result<String> {
        self.apply_formatting(code, FormatType::CodeBlock {
            language: language.map(|l| l.to_string()),
        })
    }
    
    /// Create a table with headers and rows
    pub fn create_table(&self, headers: Vec<&str>, rows: Vec<Vec<&str>>) -> Result<String> {
        let header_strings: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
        let row_strings: Vec<Vec<String>> = rows.iter()
            .map(|row| row.iter().map(|cell| cell.to_string()).collect())
            .collect();
        
        self.apply_formatting("", FormatType::Table {
            headers: header_strings,
            rows: row_strings,
        })
    }
    
    /// Toggle formatting on selected text (add if not present, remove if present)
    pub fn toggle_formatting(&self, text: &str, format: FormatType) -> Result<String> {
        match format {
            FormatType::Bold => {
                if text.starts_with("**") && text.ends_with("**") && text.len() > 4 {
                    Ok(text[2..text.len()-2].to_string())
                } else {
                    self.make_bold(text)
                }
            },
            FormatType::Italic => {
                if text.starts_with('*') && text.ends_with('*') && text.len() > 2 && !text.starts_with("**") {
                    Ok(text[1..text.len()-1].to_string())
                } else {
                    self.make_italic(text)
                }
            },
            FormatType::Strikethrough => {
                if text.starts_with("~~") && text.ends_with("~~") && text.len() > 4 {
                    Ok(text[2..text.len()-2].to_string())
                } else {
                    self.make_strikethrough(text)
                }
            },
            FormatType::Code => {
                if text.starts_with('`') && text.ends_with('`') && text.len() > 2 {
                    Ok(text[1..text.len()-1].to_string())
                } else {
                    self.make_inline_code(text)
                }
            },
            _ => self.apply_formatting(text, format),
        }
    }
    
    /// Change heading level (increase or decrease)
    pub fn change_heading_level(&self, markdown: &str, line_number: usize, delta: i8) -> Result<String> {
        let lines: Vec<&str> = markdown.lines().collect();
        if line_number >= lines.len() {
            return Ok(markdown.to_string());
        }
        
        let line = lines[line_number];
        let trimmed = line.trim();
        
        if !trimmed.starts_with('#') {
            return Ok(markdown.to_string());
        }
        
        let current_level = trimmed.chars().take_while(|&c| c == '#').count() as i8;
        let new_level = (current_level + delta).max(1).min(6) as u8;
        
        let content = trimmed.trim_start_matches('#').trim();
        let new_heading = self.make_heading(content, new_level)?;
        
        let mut updated_lines: Vec<String> = lines.iter().map(|&s| s.to_string()).collect();
        updated_lines[line_number] = new_heading;
        
        Ok(updated_lines.join("\n"))
    }
    
    /// Insert list item at specified position
    pub fn insert_list_item(&self, markdown: &str, line_number: usize, text: &str, list_type: &str) -> Result<String> {
        let list_item = match list_type {
            "unordered" => self.make_unordered_list_item(text)?,
            "ordered" => self.make_ordered_list_item(text)?,
            "task" => self.make_task_list_item(text, false)?,
            _ => return Err(anyhow::anyhow!("Unknown list type: {}", list_type)),
        };
        
        self.insert_element(markdown, line_number, &list_item)
    }
    
    /// Convert paragraph to list item
    pub fn convert_to_list_item(&self, markdown: &str, line_number: usize, list_type: &str) -> Result<String> {
        let lines: Vec<&str> = markdown.lines().collect();
        if line_number >= lines.len() {
            return Ok(markdown.to_string());
        }
        
        let line = lines[line_number];
        let content = line.trim();
        
        // Skip if already a list item
        if content.starts_with('-') || content.starts_with('*') || content.starts_with('+') || 
           (content.len() > 2 && content.chars().nth(1) == Some('.') && content.chars().nth(0).unwrap().is_ascii_digit()) {
            return Ok(markdown.to_string());
        }
        
        let list_item = match list_type {
            "unordered" => self.make_unordered_list_item(content)?,
            "ordered" => self.make_ordered_list_item(content)?,
            "task" => self.make_task_list_item(content, false)?,
            _ => return Err(anyhow::anyhow!("Unknown list type: {}", list_type)),
        };
        
        let mut updated_lines: Vec<String> = lines.iter().map(|&s| s.to_string()).collect();
        updated_lines[line_number] = list_item;
        
        Ok(updated_lines.join("\n"))
    }
    
    /// Toggle task list item completion
    pub fn toggle_task_completion(&self, markdown: &str, line_number: usize) -> Result<String> {
        let lines: Vec<&str> = markdown.lines().collect();
        if line_number >= lines.len() {
            return Ok(markdown.to_string());
        }
        
        let line = lines[line_number];
        let trimmed = line.trim();
        
        let new_line = if trimmed.starts_with("- [ ]") {
            line.replace("- [ ]", "- [x]")
        } else if trimmed.starts_with("- [x]") {
            line.replace("- [x]", "- [ ]")
        } else {
            return Ok(markdown.to_string());
        };
        
        let mut updated_lines: Vec<String> = lines.iter().map(|&s| s.to_string()).collect();
        updated_lines[line_number] = new_line;
        
        Ok(updated_lines.join("\n"))
    }
    
    /// Indent list item (increase nesting level)
    pub fn indent_list_item(&self, markdown: &str, line_number: usize) -> Result<String> {
        let lines: Vec<&str> = markdown.lines().collect();
        if line_number >= lines.len() {
            return Ok(markdown.to_string());
        }
        
        let line = lines[line_number];
        let trimmed = line.trim();
        
        // Only indent if it's a list item
        if !self.is_list_item(trimmed) {
            return Ok(markdown.to_string());
        }
        
        let indented_line = format!("  {}", line);
        let mut updated_lines: Vec<String> = lines.iter().map(|&s| s.to_string()).collect();
        updated_lines[line_number] = indented_line;
        
        Ok(updated_lines.join("\n"))
    }
    
    /// Outdent list item (decrease nesting level)
    pub fn outdent_list_item(&self, markdown: &str, line_number: usize) -> Result<String> {
        let lines: Vec<&str> = markdown.lines().collect();
        if line_number >= lines.len() {
            return Ok(markdown.to_string());
        }
        
        let line = lines[line_number];
        
        // Remove up to 2 spaces from the beginning
        let outdented_line = if line.starts_with("  ") {
            &line[2..]
        } else if line.starts_with(' ') {
            &line[1..]
        } else {
            line
        };
        
        let mut updated_lines: Vec<String> = lines.iter().map(|&s| s.to_string()).collect();
        updated_lines[line_number] = outdented_line.to_string();
        
        Ok(updated_lines.join("\n"))
    }
    
    /// Insert horizontal rule
    pub fn insert_horizontal_rule(&self, markdown: &str, line_number: usize) -> Result<String> {
        self.insert_element(markdown, line_number, "---")
    }
    
    /// Wrap selection with custom markdown syntax
    pub fn wrap_selection(&self, text: &str, prefix: &str, suffix: &str) -> String {
        format!("{}{}{}", prefix, text, suffix)
    }
    
    /// Remove formatting from text
    pub fn remove_formatting(&self, text: &str) -> String {
        text
            .replace("**", "")
            .replace("*", "")
            .replace("~~", "")
            .replace("`", "")
            .replace("# ", "")
            .replace("## ", "")
            .replace("### ", "")
            .replace("#### ", "")
            .replace("##### ", "")
            .replace("###### ", "")
            .replace("- ", "")
            .replace("+ ", "")
            .replace("> ", "")
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
    
    #[test]
    fn test_apply_formatting_bold() {
        let service = MarkdownService::new();
        let result = service.apply_formatting("text", FormatType::Bold).unwrap();
        assert_eq!(result, "**text**");
    }
    
    #[test]
    fn test_apply_formatting_italic() {
        let service = MarkdownService::new();
        let result = service.apply_formatting("text", FormatType::Italic).unwrap();
        assert_eq!(result, "*text*");
    }
    
    #[test]
    fn test_apply_formatting_heading() {
        let service = MarkdownService::new();
        let result = service.apply_formatting("Title", FormatType::Heading { level: 2 }).unwrap();
        assert_eq!(result, "## Title");
    }
    
    #[test]
    fn test_apply_formatting_link() {
        let service = MarkdownService::new();
        let result = service.apply_formatting(
            "Google", 
            FormatType::Link { 
                url: "https://google.com".to_string(), 
                title: None 
            }
        ).unwrap();
        assert_eq!(result, "[Google](https://google.com)");
    }
    
    #[test]
    fn test_apply_formatting_link_with_title() {
        let service = MarkdownService::new();
        let result = service.apply_formatting(
            "Google", 
            FormatType::Link { 
                url: "https://google.com".to_string(), 
                title: Some("Search Engine".to_string()) 
            }
        ).unwrap();
        assert_eq!(result, "[Google](https://google.com \"Search Engine\")");
    }
    
    #[test]
    fn test_apply_formatting_image() {
        let service = MarkdownService::new();
        let result = service.apply_formatting(
            "", 
            FormatType::Image { 
                url: "image.jpg".to_string(), 
                alt: "Alt text".to_string(),
                title: None 
            }
        ).unwrap();
        assert_eq!(result, "![Alt text](image.jpg)");
    }
    
    #[test]
    fn test_apply_formatting_code_block() {
        let service = MarkdownService::new();
        let result = service.apply_formatting(
            "console.log('hello');", 
            FormatType::CodeBlock { language: Some("javascript".to_string()) }
        ).unwrap();
        assert_eq!(result, "```javascript\nconsole.log('hello');\n```");
    }
    
    #[test]
    fn test_apply_formatting_table() {
        let service = MarkdownService::new();
        let headers = vec!["Name".to_string(), "Age".to_string()];
        let rows = vec![
            vec!["John".to_string(), "25".to_string()],
            vec!["Jane".to_string(), "30".to_string()],
        ];
        let result = service.apply_formatting(
            "", 
            FormatType::Table { headers, rows }
        ).unwrap();
        
        assert!(result.contains("| Name | Age |"));
        assert!(result.contains("| John | 25 |"));
        assert!(result.contains("| Jane | 30 |"));
    }
    
    #[test]
    fn test_validate_syntax_valid_markdown() {
        let service = MarkdownService::new();
        let markdown = "# Valid Heading\n\n[Valid Link](https://example.com)\n\n![Valid Image](image.jpg)";
        let errors = service.validate_syntax(markdown).unwrap();
        assert!(errors.is_empty());
    }
    
    #[test]
    fn test_validate_syntax_invalid_heading() {
        let service = MarkdownService::new();
        let markdown = "#######Invalid Heading";
        let errors = service.validate_syntax(markdown).unwrap();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].error_type, ValidationErrorType::InvalidHeading));
    }
    
    #[test]
    fn test_validate_syntax_missing_space_in_heading() {
        let service = MarkdownService::new();
        let markdown = "#InvalidHeading";
        let errors = service.validate_syntax(markdown).unwrap();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].error_type, ValidationErrorType::InvalidHeading));
    }
    
    #[test]
    fn test_validate_syntax_empty_link() {
        let service = MarkdownService::new();
        let markdown = "[Empty Link]()";
        let errors = service.validate_syntax(markdown).unwrap();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].error_type, ValidationErrorType::MalformedLink));
    }
    
    #[test]
    fn test_validate_syntax_unclosed_code_block() {
        let service = MarkdownService::new();
        let markdown = "```javascript\nconsole.log('hello');";
        let errors = service.validate_syntax(markdown).unwrap();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].error_type, ValidationErrorType::UnclosedCodeBlock));
    }
    
    #[test]
    fn test_validate_syntax_malformed_table() {
        let service = MarkdownService::new();
        let markdown = "| |";
        let errors = service.validate_syntax(markdown).unwrap();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].error_type, ValidationErrorType::MalformedTable));
    }
    
    #[test]
    fn test_complex_markdown_elements() {
        let service = MarkdownService::new();
        let markdown = r#"# Main Title

## Subtitle

This is a paragraph with **bold** and *italic* text.

### Lists

- Unordered item 1
- Unordered item 2
  - Nested item

1. Ordered item 1
2. Ordered item 2

### Task List

- [x] Completed task
- [ ] Incomplete task

### Links and Images

[Link to Google](https://google.com "Search Engine")
![Sample Image](image.jpg "Sample")

### Code

Inline `code` and block:

```javascript
function hello() {
    console.log("Hello, World!");
}
```

### Table

| Name | Age | City |
|------|-----|------|
| John | 25  | NYC  |
| Jane | 30  | LA   |

### Blockquote

> This is a blockquote
> with multiple lines
"#;
        
        let result = service.parse_to_elements(markdown).unwrap();
        
        // Verify we have various element types
        let element_types: Vec<&str> = result.elements.iter()
            .map(|e| e.element_type.as_str())
            .collect();
        
        assert!(element_types.contains(&"heading1"));
        assert!(element_types.contains(&"heading2"));
        assert!(element_types.contains(&"heading3"));
        assert!(element_types.contains(&"paragraph"));
        assert!(element_types.contains(&"unordered_list") || element_types.contains(&"list_item"));
        assert!(element_types.contains(&"task_list") || element_types.contains(&"task_item"));
        assert!(element_types.contains(&"code_block"));
        assert!(element_types.contains(&"table") || element_types.contains(&"table_row"));
        assert!(element_types.contains(&"blockquote"));
        
        // Verify metadata
        assert!(result.metadata.heading_count >= 4);
        assert!(result.metadata.link_count >= 1);
        assert!(result.metadata.image_count >= 1);
        assert!(result.metadata.word_count > 20);
    }
    
    #[test]
    fn test_multi_line_code_block_parsing() {
        let service = MarkdownService::new();
        let markdown = r#"```javascript
function test() {
    return "hello";
}
```"#;
        
        let result = service.parse_to_elements(markdown).unwrap();
        assert_eq!(result.elements.len(), 1);
        assert_eq!(result.elements[0].element_type, "code_block");
        assert!(result.elements[0].content.contains("function test()"));
        assert_eq!(result.elements[0].attributes.get("language"), Some(&"javascript".to_string()));
    }
    
    #[test]
    fn test_multi_line_blockquote_parsing() {
        let service = MarkdownService::new();
        let markdown = r#"> This is line one
> This is line two
> This is line three"#;
        
        let result = service.parse_to_elements(markdown).unwrap();
        assert_eq!(result.elements.len(), 1);
        assert_eq!(result.elements[0].element_type, "blockquote");
        assert!(result.elements[0].content.contains("This is line one"));
        assert!(result.elements[0].content.contains("This is line three"));
    }
    
    #[test]
    fn test_table_structure_parsing() {
        let service = MarkdownService::new();
        let markdown = r#"| Name | Age |
|------|-----|
| John | 25  |
| Jane | 30  |"#;
        
        let result = service.parse_to_elements(markdown).unwrap();
        assert_eq!(result.elements.len(), 1);
        assert_eq!(result.elements[0].element_type, "table");
        assert!(result.elements[0].children.len() >= 2); // At least 2 rows
    }
    
    #[test]
    fn test_list_structure_parsing() {
        let service = MarkdownService::new();
        let markdown = r#"- Item 1
- Item 2
- Item 3"#;
        
        let result = service.parse_to_elements(markdown).unwrap();
        assert_eq!(result.elements.len(), 1);
        assert_eq!(result.elements[0].element_type, "unordered_list");
        assert_eq!(result.elements[0].children.len(), 3);
    }
    
    #[test]
    fn test_element_update() {
        let service = MarkdownService::new();
        let markdown = "# Original Title\n\nSome content";
        
        let position = Position {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 16,
        };
        
        let result = service.update_element(markdown, &position, "# Updated Title").unwrap();
        assert!(result.contains("# Updated Title"));
        assert!(result.contains("Some content"));
    }
    
    #[test]
    fn test_element_insertion() {
        let service = MarkdownService::new();
        let markdown = "# Title\n\nContent";
        
        let result = service.insert_element(markdown, 1, "## Subtitle").unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "## Subtitle");
    }
    
    #[test]
    fn test_element_deletion() {
        let service = MarkdownService::new();
        let markdown = "# Title\n## Subtitle\nContent";
        
        let position = Position {
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 11,
        };
        
        let result = service.delete_element(markdown, &position).unwrap();
        assert!(!result.contains("## Subtitle"));
        assert!(result.contains("# Title"));
        assert!(result.contains("Content"));
    }
    
    #[test]
    fn test_find_elements_by_type() {
        let service = MarkdownService::new();
        let markdown = "# Title 1\n## Title 2\n### Title 3";
        
        let headings = service.find_elements_by_type(markdown, "heading1").unwrap();
        assert_eq!(headings.len(), 1);
        assert!(headings[0].content.contains("Title 1"));
    }
    
    #[test]
    fn test_get_element_at_position() {
        let service = MarkdownService::new();
        let markdown = "# Title\n\nParagraph content";
        
        let element = service.get_element_at_position(markdown, 0, 2).unwrap();
        assert!(element.is_some());
        assert_eq!(element.unwrap().element_type, "heading1");
    }
    
    #[test]
    fn test_make_bold() {
        let service = MarkdownService::new();
        let result = service.make_bold("text").unwrap();
        assert_eq!(result, "**text**");
    }
    
    #[test]
    fn test_make_italic() {
        let service = MarkdownService::new();
        let result = service.make_italic("text").unwrap();
        assert_eq!(result, "*text*");
    }
    
    #[test]
    fn test_make_heading() {
        let service = MarkdownService::new();
        let result = service.make_heading("Title", 3).unwrap();
        assert_eq!(result, "### Title");
    }
    
    #[test]
    fn test_make_heading_invalid_level() {
        let service = MarkdownService::new();
        let result = service.make_heading("Title", 7);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_create_link() {
        let service = MarkdownService::new();
        let result = service.create_link("Google", "https://google.com", Some("Search")).unwrap();
        assert_eq!(result, "[Google](https://google.com \"Search\")");
    }
    
    #[test]
    fn test_create_image() {
        let service = MarkdownService::new();
        let result = service.create_image("Alt text", "image.jpg", None).unwrap();
        assert_eq!(result, "![Alt text](image.jpg)");
    }
    
    #[test]
    fn test_make_task_list_item() {
        let service = MarkdownService::new();
        let result = service.make_task_list_item("Task", true).unwrap();
        assert_eq!(result, "- [x] Task");
        
        let result = service.make_task_list_item("Task", false).unwrap();
        assert_eq!(result, "- [ ] Task");
    }
    
    #[test]
    fn test_create_code_block() {
        let service = MarkdownService::new();
        let result = service.create_code_block("console.log('hello');", Some("javascript")).unwrap();
        assert_eq!(result, "```javascript\nconsole.log('hello');\n```");
    }
    
    #[test]
    fn test_create_table() {
        let service = MarkdownService::new();
        let headers = vec!["Name", "Age"];
        let rows = vec![vec!["John", "25"], vec!["Jane", "30"]];
        let result = service.create_table(headers, rows).unwrap();
        
        assert!(result.contains("| Name | Age |"));
        assert!(result.contains("| John | 25 |"));
        assert!(result.contains("| Jane | 30 |"));
    }
    
    #[test]
    fn test_toggle_formatting_bold() {
        let service = MarkdownService::new();
        
        // Add bold formatting
        let result = service.toggle_formatting("text", FormatType::Bold).unwrap();
        assert_eq!(result, "**text**");
        
        // Remove bold formatting
        let result = service.toggle_formatting("**text**", FormatType::Bold).unwrap();
        assert_eq!(result, "text");
    }
    
    #[test]
    fn test_toggle_formatting_italic() {
        let service = MarkdownService::new();
        
        // Add italic formatting
        let result = service.toggle_formatting("text", FormatType::Italic).unwrap();
        assert_eq!(result, "*text*");
        
        // Remove italic formatting
        let result = service.toggle_formatting("*text*", FormatType::Italic).unwrap();
        assert_eq!(result, "text");
    }
    
    #[test]
    fn test_change_heading_level() {
        let service = MarkdownService::new();
        let markdown = "## Heading\nContent";
        
        // Increase level
        let result = service.change_heading_level(markdown, 0, 1).unwrap();
        assert!(result.contains("### Heading"));
        
        // Decrease level
        let result = service.change_heading_level(markdown, 0, -1).unwrap();
        assert!(result.contains("# Heading"));
    }
    
    #[test]
    fn test_insert_list_item() {
        let service = MarkdownService::new();
        let markdown = "# Title\nContent";
        
        let result = service.insert_list_item(markdown, 1, "New item", "unordered").unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "- New item");
    }
    
    #[test]
    fn test_convert_to_list_item() {
        let service = MarkdownService::new();
        let markdown = "# Title\nRegular paragraph";
        
        let result = service.convert_to_list_item(markdown, 1, "unordered").unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "- Regular paragraph");
    }
    
    #[test]
    fn test_toggle_task_completion() {
        let service = MarkdownService::new();
        let markdown = "- [ ] Incomplete task\n- [x] Complete task";
        
        // Complete the incomplete task
        let result = service.toggle_task_completion(markdown, 0).unwrap();
        assert!(result.contains("- [x] Incomplete task"));
        
        // Incomplete the complete task
        let result = service.toggle_task_completion(markdown, 1).unwrap();
        assert!(result.contains("- [ ] Complete task"));
    }
    
    #[test]
    fn test_indent_outdent_list_item() {
        let service = MarkdownService::new();
        let markdown = "- Item 1\n- Item 2";
        
        // Indent second item
        let result = service.indent_list_item(markdown, 1).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "  - Item 2");
        
        // Outdent the indented item
        let result = service.outdent_list_item(&result, 1).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "- Item 2");
    }
    
    #[test]
    fn test_insert_horizontal_rule() {
        let service = MarkdownService::new();
        let markdown = "# Title\nContent";
        
        let result = service.insert_horizontal_rule(markdown, 1).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "---");
    }
    
    #[test]
    fn test_wrap_selection() {
        let service = MarkdownService::new();
        let result = service.wrap_selection("text", "**", "**");
        assert_eq!(result, "**text**");
    }
    
    #[test]
    fn test_remove_formatting() {
        let service = MarkdownService::new();
        let formatted_text = "**bold** and *italic* and ~~strikethrough~~ and `code`";
        let result = service.remove_formatting(formatted_text);
        assert_eq!(result, "bold and italic and strikethrough and code");
    }
}