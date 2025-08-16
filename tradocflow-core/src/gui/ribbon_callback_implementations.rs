use crate::gui::markdown_editor_bridge::MarkdownEditorBridge;
use anyhow::Result;
use slint::SharedString;

/// Extended implementation for Markdown Ribbon callbacks
/// This module provides example implementations for all ribbon formatting operations
impl MarkdownEditorBridge {
    
    // ========================================
    // TEXT FORMATTING CALLBACKS
    // ========================================
    
    /// Apply bold formatting to selected text or insert bold markers
    pub fn format_bold(&self, content: &str, selection_start: usize, selection_end: usize) -> Result<(String, usize, usize)> {
        if selection_start == selection_end {
            // No selection, insert bold markers
            let before = &content[..selection_start];
            let after = &content[selection_start..];
            let new_content = format!("{}****{}", before, after);
            let new_cursor_pos = selection_start + 2; // Position cursor between **|**
            Ok((new_content, new_cursor_pos, new_cursor_pos))
        } else {
            // Wrap selection with bold markers
            self.wrap_selection(content, selection_start, selection_end, "**", "**")
        }
    }
    
    /// Apply italic formatting to selected text or insert italic markers
    pub fn format_italic(&self, content: &str, selection_start: usize, selection_end: usize) -> Result<(String, usize, usize)> {
        if selection_start == selection_end {
            let before = &content[..selection_start];
            let after = &content[selection_start..];
            let new_content = format!("{}**{}", before, after);
            let new_cursor_pos = selection_start + 1;
            Ok((new_content, new_cursor_pos, new_cursor_pos))
        } else {
            self.wrap_selection(content, selection_start, selection_end, "*", "*")
        }
    }
    
    /// Apply strikethrough formatting to selected text
    pub fn format_strikethrough(&self, content: &str, selection_start: usize, selection_end: usize) -> Result<(String, usize, usize)> {
        if selection_start == selection_end {
            let before = &content[..selection_start];
            let after = &content[selection_start..];
            let new_content = format!("{}~~~~{}", before, after);
            let new_cursor_pos = selection_start + 2;
            Ok((new_content, new_cursor_pos, new_cursor_pos))
        } else {
            self.wrap_selection(content, selection_start, selection_end, "~~", "~~")
        }
    }
    
    /// Apply underline formatting (using HTML in markdown)
    pub fn format_underline(&self, content: &str, selection_start: usize, selection_end: usize) -> Result<(String, usize, usize)> {
        if selection_start == selection_end {
            let before = &content[..selection_start];
            let after = &content[selection_start..];
            let new_content = format!("{}<u></u>{}", before, after);
            let new_cursor_pos = selection_start + 3; // Position cursor between <u>|</u>
            Ok((new_content, new_cursor_pos, new_cursor_pos))
        } else {
            self.wrap_selection(content, selection_start, selection_end, "<u>", "</u>")
        }
    }
    
    /// Apply inline code formatting
    pub fn format_code(&self, content: &str, selection_start: usize, selection_end: usize) -> Result<(String, usize, usize)> {
        if selection_start == selection_end {
            let before = &content[..selection_start];
            let after = &content[selection_start..];
            let new_content = format!("{}``{}", before, after);
            let new_cursor_pos = selection_start + 1;
            Ok((new_content, new_cursor_pos, new_cursor_pos))
        } else {
            self.wrap_selection(content, selection_start, selection_end, "`", "`")
        }
    }
    
    // ========================================
    // STRUCTURE FORMATTING CALLBACKS
    // ========================================
    
    /// Apply heading formatting to current line
    pub fn format_heading(&self, content: &str, cursor_pos: usize, level: i32) -> Result<(String, usize)> {
        let level = level.clamp(1, 6) as usize;
        let heading_prefix = "#".repeat(level) + " ";
        
        // Find the start of the current line
        let line_start = content[..cursor_pos].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
        
        // Check if line already has heading markers
        let line_end = content[cursor_pos..].find('\n').map(|pos| cursor_pos + pos).unwrap_or(content.len());
        let current_line = &content[line_start..line_end];
        
        if let Some(existing_heading) = current_line.strip_prefix('#') {
            // Replace existing heading
            let content_start = existing_heading.trim_start_matches('#').trim_start();
            let before = &content[..line_start];
            let after = &content[line_start + current_line.len()..];
            let new_content = format!("{}{}{}{}", before, heading_prefix, content_start, after);
            let new_cursor_pos = line_start + heading_prefix.len() + content_start.len();
            Ok((new_content, new_cursor_pos))
        } else {
            // Add new heading
            let before = &content[..line_start];
            let after = &content[line_start..];
            let new_content = format!("{}{}{}", before, heading_prefix, after);
            let new_cursor_pos = cursor_pos + heading_prefix.len();
            Ok((new_content, new_cursor_pos))
        }
    }
    
    /// Apply blockquote formatting to current line or selection
    pub fn format_quote(&self, content: &str, cursor_pos: usize) -> Result<(String, usize)> {
        let line_start = content[..cursor_pos].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
        let line_end = content[cursor_pos..].find('\n').map(|pos| cursor_pos + pos).unwrap_or(content.len());
        let current_line = &content[line_start..line_end];
        
        if current_line.starts_with("> ") {
            // Remove existing blockquote
            let before = &content[..line_start];
            let after = &content[line_start + 2..];
            let new_content = format!("{}{}", before, after);
            let new_cursor_pos = cursor_pos.saturating_sub(2);
            Ok((new_content, new_cursor_pos))
        } else {
            // Add blockquote
            let before = &content[..line_start];
            let after = &content[line_start..];
            let new_content = format!("{}> {}", before, after);
            let new_cursor_pos = cursor_pos + 2;
            Ok((new_content, new_cursor_pos))
        }
    }
    
    // ========================================
    // LIST CALLBACKS
    // ========================================
    
    /// Insert bullet list item
    pub fn insert_bullet_list(&self, content: &str, cursor_pos: usize) -> Result<(String, usize)> {
        self.insert_list_item(content, cursor_pos, "- ")
    }
    
    /// Insert numbered list item
    pub fn insert_numbered_list(&self, content: &str, cursor_pos: usize) -> Result<(String, usize)> {
        // Find the appropriate number for the list item
        let list_number = self.get_next_list_number(content, cursor_pos);
        let prefix = format!("{}. ", list_number);
        self.insert_list_item(content, cursor_pos, &prefix)
    }
    
    /// Insert checklist item
    pub fn insert_checklist(&self, content: &str, cursor_pos: usize) -> Result<(String, usize)> {
        self.insert_list_item(content, cursor_pos, "- [ ] ")
    }
    
    // ========================================
    // INSERT CALLBACKS
    // ========================================
    
    /// Insert link markdown
    pub fn insert_link(&self, content: &str, cursor_pos: usize, url: Option<&str>, text: Option<&str>) -> Result<(String, usize, usize)> {
        let link_text = text.unwrap_or("Link Text");
        let link_url = url.unwrap_or("https://example.com");
        let link_markdown = format!("[{}]({})", link_text, link_url);
        
        let before = &content[..cursor_pos];
        let after = &content[cursor_pos..];
        let new_content = format!("{}{}{}", before, link_markdown, after);
        
        // Select the link text for easy editing
        let text_start = cursor_pos + 1; // After the '['
        let text_end = text_start + link_text.len();
        
        Ok((new_content, text_start, text_end))
    }
    
    /// Insert image markdown
    pub fn insert_image(&self, content: &str, cursor_pos: usize, url: Option<&str>, alt_text: Option<&str>) -> Result<(String, usize, usize)> {
        let alt = alt_text.unwrap_or("Alt Text");
        let image_url = url.unwrap_or("image-url");
        let image_markdown = format!("![{}]({})", alt, image_url);
        
        let before = &content[..cursor_pos];
        let after = &content[cursor_pos..];
        let new_content = format!("{}{}{}", before, image_markdown, after);
        
        // Select the alt text for easy editing
        let alt_start = cursor_pos + 2; // After the '!['
        let alt_end = alt_start + alt.len();
        
        Ok((new_content, alt_start, alt_end))
    }
    
    /// Insert table markdown
    pub fn insert_table(&self, content: &str, cursor_pos: usize, rows: usize, cols: usize) -> Result<(String, usize)> {
        let rows = rows.max(2); // At least header + 1 data row
        let cols = cols.max(2); // At least 2 columns
        
        let mut table = String::new();
        
        // Header row
        table.push_str("| ");
        for i in 1..=cols {
            table.push_str(&format!("Column {} ", i));
            if i < cols { table.push_str("| "); }
        }
        table.push_str("|\n");
        
        // Separator row
        table.push_str("|");
        for _ in 0..cols {
            table.push_str("----------|");
        }
        table.push('\n');
        
        // Data rows
        for row in 1..(rows) {
            table.push_str("| ");
            for col in 1..=cols {
                table.push_str(&format!("Cell {},{} ", row, col));
                if col < cols { table.push_str("| "); }
            }
            table.push_str("|\n");
        }
        
        let before = &content[..cursor_pos];
        let after = &content[cursor_pos..];
        let new_content = format!("{}\n{}\n{}", before, table.trim(), after);
        let new_cursor_pos = cursor_pos + table.len() + 2;
        
        Ok((new_content, new_cursor_pos))
    }
    
    /// Insert code block
    pub fn insert_code_block(&self, content: &str, cursor_pos: usize, language: Option<&str>) -> Result<(String, usize, usize)> {
        let lang = language.unwrap_or("");
        let code_block = format!("```{}\n\n```", lang);
        
        let before = &content[..cursor_pos];
        let after = &content[cursor_pos..];
        let new_content = format!("{}\n{}\n{}", before, code_block, after);
        
        // Position cursor inside the code block
        let code_start = cursor_pos + 4 + lang.len() + 1; // After "```lang\n"
        let code_end = code_start;
        
        Ok((new_content, code_start, code_end))
    }
    
    /// Insert horizontal rule
    pub fn insert_horizontal_rule(&self, content: &str, cursor_pos: usize) -> Result<(String, usize)> {
        let before = &content[..cursor_pos];
        let after = &content[cursor_pos..];
        let new_content = format!("{}\n\n---\n\n{}", before, after);
        let new_cursor_pos = cursor_pos + 7; // After "\n\n---\n\n"
        
        Ok((new_content, new_cursor_pos))
    }
    
    // ========================================
    // TEXT MANIPULATION CALLBACKS
    // ========================================
    
    /// Increase indentation of current line or selection
    pub fn increase_indent(&self, content: &str, cursor_pos: usize) -> Result<(String, usize)> {
        self.modify_indentation(content, cursor_pos, true)
    }
    
    /// Decrease indentation of current line or selection
    pub fn decrease_indent(&self, content: &str, cursor_pos: usize) -> Result<(String, usize)> {
        self.modify_indentation(content, cursor_pos, false)
    }
    
    // ========================================
    // HELPER METHODS
    // ========================================
    
    /// Wrap selected text with prefix and suffix
    fn wrap_selection(&self, content: &str, start: usize, end: usize, prefix: &str, suffix: &str) -> Result<(String, usize, usize)> {
        let before = &content[..start];
        let selected = &content[start..end];
        let after = &content[end..];
        
        let new_content = format!("{}{}{}{}{}", before, prefix, selected, suffix, after);
        let new_start = start + prefix.len();
        let new_end = new_start + selected.len();
        
        Ok((new_content, new_start, new_end))
    }
    
    /// Insert list item at current position
    fn insert_list_item(&self, content: &str, cursor_pos: usize, prefix: &str) -> Result<(String, usize)> {
        // Check if we're at the beginning of a line or need to create a new line
        let needs_newline = cursor_pos > 0 && !content[..cursor_pos].ends_with('\n');
        
        let before = &content[..cursor_pos];
        let after = &content[cursor_pos..];
        
        let new_content = if needs_newline {
            format!("{}\n{}{}", before, prefix, after)
        } else {
            format!("{}{}{}", before, prefix, after)
        };
        
        let new_cursor_pos = cursor_pos + prefix.len() + if needs_newline { 1 } else { 0 };
        
        Ok((new_content, new_cursor_pos))
    }
    
    /// Get the next number for a numbered list
    fn get_next_list_number(&self, content: &str, cursor_pos: usize) -> usize {
        // Find previous numbered list items to determine the next number
        let lines_before = content[..cursor_pos].lines().rev();
        
        for line in lines_before {
            let trimmed = line.trim_start();
            if let Some(after_digit) = trimmed.strip_prefix(char::is_numeric) {
                if after_digit.starts_with(". ") {
                    // Found a numbered list item, extract the number
                    if let Some(number_str) = trimmed.split('.').next() {
                        if let Ok(number) = number_str.parse::<usize>() {
                            return number + 1;
                        }
                    }
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with("- ") && !trimmed.starts_with("* ") {
                // Hit non-list content, start from 1
                break;
            }
        }
        
        1 // Default starting number
    }
    
    /// Modify indentation of current line
    fn modify_indentation(&self, content: &str, cursor_pos: usize, increase: bool) -> Result<(String, usize)> {
        let line_start = content[..cursor_pos].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
        let line_end = content[cursor_pos..].find('\n').map(|pos| cursor_pos + pos).unwrap_or(content.len());
        let current_line = &content[line_start..line_end];
        
        let (new_line, cursor_adjustment) = if increase {
            // Add 4 spaces of indentation
            (format!("    {}", current_line), 4i32)
        } else {
            // Remove up to 4 spaces of indentation
            if current_line.starts_with("    ") {
                (current_line[4..].to_string(), -4i32)
            } else if current_line.starts_with("   ") {
                (current_line[3..].to_string(), -3i32)
            } else if current_line.starts_with("  ") {
                (current_line[2..].to_string(), -2i32)
            } else if current_line.starts_with(" ") {
                (current_line[1..].to_string(), -1i32)
            } else {
                (current_line.to_string(), 0i32)
            }
        };
        
        let before = &content[..line_start];
        let after = &content[line_end..];
        let new_content = format!("{}{}{}", before, new_line, after);
        let new_cursor_pos = (cursor_pos as i32 + cursor_adjustment).max(line_start as i32) as usize;
        
        Ok((new_content, new_cursor_pos))
    }
    
    // ========================================
    // STATE MANAGEMENT HELPERS
    // ========================================
    
    /// Check if undo operation is available
    pub fn can_undo(&self) -> bool {
        // Implement based on your undo stack
        false // Placeholder
    }
    
    /// Check if redo operation is available  
    pub fn can_redo(&self) -> bool {
        // Implement based on your redo stack
        false // Placeholder
    }
    
    /// Check if there is selected text
    pub fn has_selection(&self, selection_start: usize, selection_end: usize) -> bool {
        selection_start != selection_end
    }
    
    /// Update ribbon state based on current editor state
    pub fn update_ribbon_state(&self, content: &str, cursor_pos: usize, selection_start: usize, selection_end: usize) -> RibbonState {
        RibbonState {
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            has_selection: self.has_selection(selection_start, selection_end),
            current_heading_level: self.get_current_heading_level(content, cursor_pos),
            in_list: self.is_in_list(content, cursor_pos),
        }
    }
    
    /// Get the heading level of the current line (0 if not a heading)
    fn get_current_heading_level(&self, content: &str, cursor_pos: usize) -> i32 {
        let line_start = content[..cursor_pos].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
        let line_end = content[cursor_pos..].find('\n').map(|pos| cursor_pos + pos).unwrap_or(content.len());
        let current_line = &content[line_start..line_end];
        
        let trimmed = current_line.trim_start();
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|&c| c == '#').count();
            if level <= 6 && trimmed.chars().nth(level) == Some(' ') {
                return level as i32;
            }
        }
        
        0
    }
    
    /// Check if cursor is currently in a list
    fn is_in_list(&self, content: &str, cursor_pos: usize) -> bool {
        let line_start = content[..cursor_pos].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
        let line_end = content[cursor_pos..].find('\n').map(|pos| cursor_pos + pos).unwrap_or(content.len());
        let current_line = &content[line_start..line_end];
        
        let trimmed = current_line.trim_start();
        trimmed.starts_with("- ") || 
        trimmed.starts_with("* ") || 
        trimmed.starts_with("+ ") ||
        trimmed.starts_with("- [ ]") ||
        trimmed.starts_with("- [x]") ||
        (trimmed.chars().next().map_or(false, |c| c.is_ascii_digit()) && trimmed.contains(". "))
    }
}

/// State information for updating ribbon UI
#[derive(Debug, Clone)]
pub struct RibbonState {
    pub can_undo: bool,
    pub can_redo: bool,
    pub has_selection: bool,
    pub current_heading_level: i32,
    pub in_list: bool,
}

// ========================================
// EXAMPLE USAGE IN SLINT INTEGRATION
// ========================================

/// Example of how to integrate these callbacks with Slint
/// This would typically be in your main application logic
pub struct RibbonIntegration {
    bridge: MarkdownEditorBridge,
}

impl RibbonIntegration {
    pub fn new() -> Self {
        Self {
            bridge: MarkdownEditorBridge::new(),
        }
    }
    
    /// Example callback handler for bold formatting
    pub fn handle_format_bold(&self, content: SharedString, selection_start: i32, selection_end: i32) -> Result<(SharedString, i32, i32)> {
        let (new_content, new_start, new_end) = self.bridge.format_bold(
            &content.to_string(),
            selection_start as usize,
            selection_end as usize,
        )?;
        
        Ok((
            SharedString::from(new_content),
            new_start as i32,
            new_end as i32,
        ))
    }
    
    /// Example callback handler for heading formatting
    pub fn handle_format_heading(&self, content: SharedString, cursor_pos: i32, level: i32) -> Result<(SharedString, i32)> {
        let (new_content, new_cursor_pos) = self.bridge.format_heading(
            &content.to_string(),
            cursor_pos as usize,
            level,
        )?;
        
        Ok((
            SharedString::from(new_content),
            new_cursor_pos as i32,
        ))
    }
    
    /// Update ribbon state and return values for Slint properties
    pub fn get_ribbon_state(&self, content: SharedString, cursor_pos: i32, selection_start: i32, selection_end: i32) -> RibbonState {
        self.bridge.update_ribbon_state(
            &content.to_string(),
            cursor_pos as usize,
            selection_start as usize,
            selection_end as usize,
        )
    }
}