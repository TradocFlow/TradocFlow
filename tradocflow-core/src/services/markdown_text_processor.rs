use std::collections::{HashMap, VecDeque};
use std::ops::Range;
use regex::{Regex, RegexBuilder};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};

/// Position in text representing cursor or selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPosition {
    pub line: u32,
    pub column: u32,
    pub offset: usize,
}

impl TextPosition {
    pub fn new(line: u32, column: u32, offset: usize) -> Self {
        Self { line, column, offset }
    }

    pub fn zero() -> Self {
        Self { line: 0, column: 0, offset: 0 }
    }
}

/// Text selection range
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSelection {
    pub start: TextPosition,
    pub end: TextPosition,
    pub is_reversed: bool, // True if selection was made backwards
}

impl TextSelection {
    pub fn new(start: TextPosition, end: TextPosition) -> Self {
        let is_reversed = start.offset > end.offset;
        Self {
            start: if is_reversed { end } else { start },
            end: if is_reversed { start } else { end },
            is_reversed,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start.offset == self.end.offset
    }

    pub fn len(&self) -> usize {
        self.end.offset - self.start.offset
    }

    pub fn contains(&self, position: &TextPosition) -> bool {
        position.offset >= self.start.offset && position.offset <= self.end.offset
    }

    pub fn intersects(&self, other: &TextSelection) -> bool {
        !(self.end.offset <= other.start.offset || other.end.offset <= self.start.offset)
    }
}

/// Cursor information for multi-cursor editing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub id: Uuid,
    pub position: TextPosition,
    pub selection: Option<TextSelection>,
    pub is_primary: bool,
}

impl Cursor {
    pub fn new(position: TextPosition) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            selection: None,
            is_primary: true,
        }
    }

    pub fn with_selection(position: TextPosition, selection: TextSelection) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            selection: Some(selection),
            is_primary: true,
        }
    }
}

/// Text operation for undo/redo functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextOperation {
    Insert {
        position: usize,
        text: String,
        cursor_positions: Vec<Cursor>,
    },
    Delete {
        position: usize,
        text: String,
        cursor_positions: Vec<Cursor>,
    },
    Replace {
        start: usize,
        end: usize,
        old_text: String,
        new_text: String,
        cursor_positions: Vec<Cursor>,
    },
    FormatApply {
        selections: Vec<TextSelection>,
        format_type: MarkdownFormat,
        cursor_positions: Vec<Cursor>,
    },
    FormatRemove {
        selections: Vec<TextSelection>,
        format_type: MarkdownFormat,
        cursor_positions: Vec<Cursor>,
    },
}

/// Markdown formatting types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkdownFormat {
    Bold,
    Italic,
    Code,
    CodeBlock { language: Option<String> },
    Heading { level: u8 },
    Link { url: String, title: Option<String> },
    Image { url: String, alt: String, title: Option<String> },
    Strikethrough,
    Underline,
    BlockQuote,
    UnorderedList,
    OrderedList,
    Table,
    HorizontalRule,
}

/// Find and replace options
#[derive(Debug, Clone)]
pub struct FindReplaceOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub multiline: bool,
    pub scope: SearchScope,
}

impl Default for FindReplaceOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            multiline: true,
            scope: SearchScope::EntireDocument,
        }
    }
}

/// Search scope for find operations
#[derive(Debug, Clone, PartialEq)]
pub enum SearchScope {
    EntireDocument,
    Selection,
    CurrentLine,
    FromCursor,
}

/// Match result from find operation
#[derive(Debug, Clone, PartialEq)]
pub struct FindMatch {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub line: u32,
    pub column: u32,
}

/// Main text processor service
pub struct MarkdownTextProcessor {
    content: String,
    cursors: Vec<Cursor>,
    undo_stack: VecDeque<TextOperation>,
    redo_stack: VecDeque<TextOperation>,
    max_undo_levels: usize,
    line_cache: Vec<Range<usize>>, // Cache of line start/end positions
    last_cache_update: Option<Instant>,
    cache_dirty: bool,
}

impl MarkdownTextProcessor {
    /// Create a new text processor
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursors: vec![Cursor::new(TextPosition::zero())],
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_undo_levels: 1000,
            line_cache: Vec::new(),
            last_cache_update: None,
            cache_dirty: true,
        }
    }

    /// Create with initial content
    pub fn with_content(content: String) -> Self {
        let mut processor = Self::new();
        processor.set_content(content);
        processor
    }

    /// Set the entire content
    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.invalidate_cache();
        self.update_cursor_positions();
    }

    /// Get the current content
    pub fn get_content(&self) -> &str {
        &self.content
    }

    /// Get current cursors
    pub fn get_cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Set cursor positions
    pub fn set_cursors(&mut self, cursors: Vec<Cursor>) {
        self.cursors = cursors;
        if self.cursors.is_empty() {
            self.cursors.push(Cursor::new(TextPosition::zero()));
        }
    }

    /// Add a new cursor at position
    pub fn add_cursor(&mut self, position: TextPosition) -> Uuid {
        let cursor = Cursor::new(position);
        let id = cursor.id;
        self.cursors.push(cursor);
        id
    }

    /// Remove cursor by ID
    pub fn remove_cursor(&mut self, cursor_id: Uuid) -> bool {
        if self.cursors.len() <= 1 {
            return false; // Always keep at least one cursor
        }
        
        let initial_len = self.cursors.len();
        self.cursors.retain(|c| c.id != cursor_id);
        self.cursors.len() < initial_len
    }

    /// Update line cache for efficient line operations
    fn update_line_cache(&mut self) {
        if !self.cache_dirty {
            return;
        }

        self.line_cache.clear();
        let mut line_start = 0;
        
        for (i, ch) in self.content.char_indices() {
            if ch == '\n' {
                self.line_cache.push(line_start..i + 1);
                line_start = i + 1;
            }
        }
        
        // Add the last line if it doesn't end with newline
        if line_start < self.content.len() {
            self.line_cache.push(line_start..self.content.len());
        }
        
        self.cache_dirty = false;
        self.last_cache_update = Some(Instant::now());
    }

    /// Invalidate the line cache
    fn invalidate_cache(&mut self) {
        self.cache_dirty = true;
    }

    /// Convert offset to line/column position
    pub fn offset_to_position(&mut self, offset: usize) -> TextPosition {
        self.update_line_cache();
        
        let clamped_offset = offset.min(self.content.len());
        
        for (line_num, line_range) in self.line_cache.iter().enumerate() {
            if line_range.contains(&clamped_offset) {
                let column = clamped_offset - line_range.start;
                return TextPosition::new(line_num as u32, column as u32, clamped_offset);
            }
        }
        
        // Fallback for edge cases
        TextPosition::new(0, 0, 0)
    }

    /// Convert line/column to offset
    pub fn position_to_offset(&mut self, position: &TextPosition) -> usize {
        self.update_line_cache();
        
        if (position.line as usize) < self.line_cache.len() {
            let line_range = &self.line_cache[position.line as usize];
            let line_start = line_range.start;
            let line_length = line_range.len();
            (line_start + (position.column as usize).min(line_length)).min(self.content.len())
        } else {
            self.content.len()
        }
    }

    /// Insert text at position
    pub fn insert_text(&mut self, position: usize, text: &str) -> Result<(), TextProcessorError> {
        if position > self.content.len() {
            return Err(TextProcessorError::InvalidPosition(position));
        }

        let old_cursors = self.cursors.clone();
        
        // Perform insertion
        self.content.insert_str(position, text);
        self.invalidate_cache();
        
        // Update cursor positions after insertion
        self.update_cursors_after_insert(position, text.len());
        
        // Add to undo stack
        let operation = TextOperation::Insert {
            position,
            text: text.to_string(),
            cursor_positions: old_cursors,
        };
        self.push_undo_operation(operation);
        
        Ok(())
    }

    /// Delete text range
    pub fn delete_range(&mut self, start: usize, end: usize) -> Result<String, TextProcessorError> {
        if start > end || end > self.content.len() {
            return Err(TextProcessorError::InvalidRange(start, end));
        }

        let old_cursors = self.cursors.clone();
        let deleted_text = self.content[start..end].to_string();
        
        // Perform deletion
        self.content.drain(start..end);
        self.invalidate_cache();
        
        // Update cursor positions after deletion
        self.update_cursors_after_delete(start, end - start);
        
        // Add to undo stack
        let operation = TextOperation::Delete {
            position: start,
            text: deleted_text.clone(),
            cursor_positions: old_cursors,
        };
        self.push_undo_operation(operation);
        
        Ok(deleted_text)
    }

    /// Replace text in range
    pub fn replace_range(&mut self, start: usize, end: usize, new_text: &str) -> Result<String, TextProcessorError> {
        if start > end || end > self.content.len() {
            return Err(TextProcessorError::InvalidRange(start, end));
        }

        let old_cursors = self.cursors.clone();
        let old_text = self.content[start..end].to_string();
        
        // Perform replacement
        self.content.replace_range(start..end, new_text);
        self.invalidate_cache();
        
        // Update cursor positions
        let old_len = end - start;
        let new_len = new_text.len();
        if new_len != old_len {
            self.update_cursors_after_replace(start, old_len, new_len);
        }
        
        // Add to undo stack
        let operation = TextOperation::Replace {
            start,
            end,
            old_text: old_text.clone(),
            new_text: new_text.to_string(),
            cursor_positions: old_cursors,
        };
        self.push_undo_operation(operation);
        
        Ok(old_text)
    }

    /// Find text with options
    pub fn find_text(&mut self, pattern: &str, options: &FindReplaceOptions) -> Result<Vec<FindMatch>, TextProcessorError> {
        let search_text = match options.scope {
            SearchScope::EntireDocument => &self.content,
            SearchScope::Selection => {
                if let Some(primary_cursor) = self.cursors.iter().find(|c| c.is_primary) {
                    if let Some(selection) = &primary_cursor.selection {
                        &self.content[selection.start.offset..selection.end.offset]
                    } else {
                        return Ok(Vec::new());
                    }
                } else {
                    return Ok(Vec::new());
                }
            },
            SearchScope::CurrentLine => {
                self.update_line_cache();
                if let Some(primary_cursor) = self.cursors.iter().find(|c| c.is_primary) {
                    let line_idx = primary_cursor.position.line as usize;
                    if line_idx < self.line_cache.len() {
                        let line_range = &self.line_cache[line_idx];
                        &self.content[line_range.clone()]
                    } else {
                        return Ok(Vec::new());
                    }
                } else {
                    return Ok(Vec::new());
                }
            },
            SearchScope::FromCursor => {
                if let Some(primary_cursor) = self.cursors.iter().find(|c| c.is_primary) {
                    &self.content[primary_cursor.position.offset..]
                } else {
                    &self.content
                }
            },
        };

        let mut matches = Vec::new();
        
        if options.use_regex {
            let regex_pattern = if options.whole_word {
                format!(r"\b{}\b", regex::escape(pattern))
            } else {
                pattern.to_string()
            };
            
            let regex = RegexBuilder::new(&regex_pattern)
                .case_insensitive(!options.case_sensitive)
                .multi_line(options.multiline)
                .build()
                .map_err(|e| TextProcessorError::RegexError(e.to_string()))?;
            
            // Collect all matches first to avoid borrowing conflicts
            let found_matches: Vec<_> = regex.find_iter(search_text)
                .map(|mat| (mat.start(), mat.end(), mat.as_str().to_string()))
                .collect();
                
            for (start_offset, end_offset, text) in found_matches {
                let position = self.offset_to_position(start_offset);
                
                matches.push(FindMatch {
                    start: start_offset,
                    end: end_offset,
                    text,
                    line: position.line,
                    column: position.column,
                });
            }
        } else {
            let search_pattern = if options.case_sensitive {
                pattern.to_string()
            } else {
                pattern.to_lowercase()
            };
            
            let search_in = if options.case_sensitive {
                search_text.to_string()
            } else {
                search_text.to_lowercase()
            };
            
            // Collect all positions first to avoid borrowing conflicts
            let mut found_positions = Vec::new();
            let mut start_pos = 0;
            
            while let Some(found_pos) = search_in[start_pos..].find(&search_pattern) {
                let actual_pos = start_pos + found_pos;
                let end_pos = actual_pos + pattern.len();
                
                // Check whole word constraint
                if options.whole_word {
                    let before_ok = actual_pos == 0 || 
                        !search_text.chars().nth(actual_pos - 1).unwrap_or(' ').is_alphanumeric();
                    let after_ok = end_pos >= search_text.len() || 
                        !search_text.chars().nth(end_pos).unwrap_or(' ').is_alphanumeric();
                    
                    if !before_ok || !after_ok {
                        start_pos = actual_pos + 1;
                        continue;
                    }
                }
                
                found_positions.push((actual_pos, end_pos, search_text[actual_pos..end_pos].to_string()));
                start_pos = actual_pos + 1;
            }
            
            // Process all found positions
            for (actual_pos, end_pos, text) in found_positions {
                let position = self.offset_to_position(actual_pos);
                matches.push(FindMatch {
                    start: actual_pos,
                    end: end_pos,
                    text,
                    line: position.line,
                    column: position.column,
                });
            }
        }
        
        Ok(matches)
    }

    /// Replace all occurrences
    pub fn replace_all(&mut self, pattern: &str, replacement: &str, options: &FindReplaceOptions) -> Result<u32, TextProcessorError> {
        let matches = self.find_text(pattern, options)?;
        let mut replace_count = 0;
        
        // Process matches in reverse order to maintain correct positions
        for mat in matches.iter().rev() {
            self.replace_range(mat.start, mat.end, replacement)?;
            replace_count += 1;
        }
        
        Ok(replace_count)
    }

    /// Apply markdown formatting to current selections
    pub fn apply_formatting(&mut self, format: MarkdownFormat) -> Result<(), TextProcessorError> {
        let selections: Vec<TextSelection> = self.cursors.iter()
            .filter_map(|c| c.selection.clone())
            .collect();
        
        if selections.is_empty() {
            return Ok(());
        }
        
        let old_cursors = self.cursors.clone();
        
        // Apply formatting in reverse order to maintain positions
        for selection in selections.iter().rev() {
            self.apply_format_to_selection(selection, &format)?;
        }
        
        // Add to undo stack
        let operation = TextOperation::FormatApply {
            selections,
            format_type: format,
            cursor_positions: old_cursors,
        };
        self.push_undo_operation(operation);
        
        Ok(())
    }

    /// Apply format to a specific selection
    fn apply_format_to_selection(&mut self, selection: &TextSelection, format: &MarkdownFormat) -> Result<(), TextProcessorError> {
        let selected_text = &self.content[selection.start.offset..selection.end.offset];
        
        let formatted_text = match format {
            MarkdownFormat::Bold => format!("**{}**", selected_text),
            MarkdownFormat::Italic => format!("*{}*", selected_text),
            MarkdownFormat::Code => format!("`{}`", selected_text),
            MarkdownFormat::CodeBlock { language } => {
                let lang = language.as_deref().unwrap_or("");
                format!("```{}\n{}\n```", lang, selected_text)
            },
            MarkdownFormat::Heading { level } => {
                let hash_count = "#".repeat(*level as usize);
                format!("{} {}", hash_count, selected_text)
            },
            MarkdownFormat::Link { url, title } => {
                if let Some(title_text) = title {
                    format!("[{}]({} \"{}\")", selected_text, url, title_text)
                } else {
                    format!("[{}]({})", selected_text, url)
                }
            },
            MarkdownFormat::Image { url, alt, title } => {
                if let Some(title_text) = title {
                    format!("![{}]({} \"{}\")", alt, url, title_text)
                } else {
                    format!("![{}]({})", alt, url)
                }
            },
            MarkdownFormat::Strikethrough => format!("~~{}~~", selected_text),
            MarkdownFormat::Underline => format!("<u>{}</u>", selected_text),
            MarkdownFormat::BlockQuote => {
                selected_text.lines()
                    .map(|line| format!("> {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            MarkdownFormat::UnorderedList => {
                selected_text.lines()
                    .map(|line| format!("- {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            MarkdownFormat::OrderedList => {
                selected_text.lines()
                    .enumerate()
                    .map(|(i, line)| format!("{}. {}", i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            MarkdownFormat::HorizontalRule => "---".to_string(),
            MarkdownFormat::Table => {
                // Simple table formatting - could be enhanced
                format!("| {} |\n|---|\n", selected_text)
            },
        };
        
        self.replace_range(selection.start.offset, selection.end.offset, &formatted_text)?;
        Ok(())
    }

    /// Undo last operation
    pub fn undo(&mut self) -> Result<bool, TextProcessorError> {
        if let Some(operation) = self.undo_stack.pop_back() {
            self.apply_undo_operation(&operation)?;
            self.redo_stack.push_back(operation);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Redo last undone operation
    pub fn redo(&mut self) -> Result<bool, TextProcessorError> {
        if let Some(operation) = self.redo_stack.pop_back() {
            self.apply_redo_operation(&operation)?;
            self.undo_stack.push_back(operation);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get undo stack depth
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get redo stack depth
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear undo/redo stacks
    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Helper methods for cursor updates
    fn update_cursor_positions(&mut self) {
        // Collect offsets to update to avoid borrowing conflicts
        let offsets: Vec<(usize, usize)> = self.cursors.iter()
            .enumerate()
            .map(|(i, cursor)| (i, cursor.position.offset))
            .collect();
            
        for (i, offset) in offsets {
            self.cursors[i].position = self.offset_to_position(offset);
        }
    }

    fn update_cursors_after_insert(&mut self, position: usize, length: usize) {
        // Collect offsets to update to avoid borrowing conflicts
        let mut cursor_updates = Vec::new();
        for (i, cursor) in self.cursors.iter().enumerate() {
            let mut cursor_offset = cursor.position.offset;
            let mut selection_updates = None;
            
            if cursor.position.offset >= position {
                cursor_offset += length;
            }
            
            if let Some(ref selection) = cursor.selection {
                let mut start_offset = selection.start.offset;
                let mut end_offset = selection.end.offset;
                
                if selection.start.offset >= position {
                    start_offset += length;
                }
                if selection.end.offset >= position {
                    end_offset += length;
                }
                
                selection_updates = Some((start_offset, end_offset));
            }
            
            cursor_updates.push((i, cursor_offset, selection_updates));
        }
        
        // Apply updates
        for (i, cursor_offset, selection_updates) in cursor_updates {
            let new_position = self.offset_to_position(cursor_offset);
            self.cursors[i].position.offset = cursor_offset;
            self.cursors[i].position = new_position;
            
            if let Some((start_offset, end_offset)) = selection_updates {
                let start_position = self.offset_to_position(start_offset);
                let end_position = self.offset_to_position(end_offset);
                
                if let Some(ref mut selection) = self.cursors[i].selection {
                    selection.start.offset = start_offset;
                    selection.end.offset = end_offset;
                    selection.start = start_position;
                    selection.end = end_position;
                }
            }
        }
    }

    fn update_cursors_after_delete(&mut self, position: usize, length: usize) {
        // Collect offsets to update to avoid borrowing conflicts
        let mut cursor_updates = Vec::new();
        for (i, cursor) in self.cursors.iter().enumerate() {
            let mut cursor_offset = cursor.position.offset;
            let mut selection_updates = None;
            
            if cursor.position.offset > position + length {
                cursor_offset -= length;
            } else if cursor.position.offset > position {
                cursor_offset = position;
            }
            
            if let Some(ref selection) = cursor.selection {
                let mut start_offset = selection.start.offset;
                let mut end_offset = selection.end.offset;
                
                if selection.start.offset > position + length {
                    start_offset -= length;
                } else if selection.start.offset > position {
                    start_offset = position;
                }
                
                if selection.end.offset > position + length {
                    end_offset -= length;
                } else if selection.end.offset > position {
                    end_offset = position;
                }
                
                selection_updates = Some((start_offset, end_offset));
            }
            
            cursor_updates.push((i, cursor_offset, selection_updates));
        }
        
        // Apply updates
        for (i, cursor_offset, selection_updates) in cursor_updates {
            let new_position = self.offset_to_position(cursor_offset);
            self.cursors[i].position.offset = cursor_offset;
            self.cursors[i].position = new_position;
            
            if let Some((start_offset, end_offset)) = selection_updates {
                let start_position = self.offset_to_position(start_offset);
                let end_position = self.offset_to_position(end_offset);
                
                if let Some(ref mut selection) = self.cursors[i].selection {
                    selection.start.offset = start_offset;
                    selection.end.offset = end_offset;
                    selection.start = start_position;
                    selection.end = end_position;
                }
            }
        }
    }

    fn update_cursors_after_replace(&mut self, position: usize, old_length: usize, new_length: usize) {
        let length_diff = new_length as i32 - old_length as i32;
        
        // Collect offsets to update to avoid borrowing conflicts
        let mut cursor_updates = Vec::new();
        for (i, cursor) in self.cursors.iter().enumerate() {
            let mut cursor_offset = cursor.position.offset;
            let mut selection_updates = None;
            
            if cursor.position.offset > position + old_length {
                cursor_offset = (cursor.position.offset as i32 + length_diff).max(0) as usize;
            } else if cursor.position.offset > position {
                cursor_offset = position + new_length;
            }
            
            if let Some(ref selection) = cursor.selection {
                let mut start_offset = selection.start.offset;
                let mut end_offset = selection.end.offset;
                
                if selection.start.offset > position + old_length {
                    start_offset = (selection.start.offset as i32 + length_diff).max(0) as usize;
                } else if selection.start.offset > position {
                    start_offset = position + new_length;
                }
                
                if selection.end.offset > position + old_length {
                    end_offset = (selection.end.offset as i32 + length_diff).max(0) as usize;
                } else if selection.end.offset > position {
                    end_offset = position + new_length;
                }
                
                selection_updates = Some((start_offset, end_offset));
            }
            
            cursor_updates.push((i, cursor_offset, selection_updates));
        }
        
        // Apply updates
        for (i, cursor_offset, selection_updates) in cursor_updates {
            let new_position = self.offset_to_position(cursor_offset);
            self.cursors[i].position.offset = cursor_offset;
            self.cursors[i].position = new_position;
            
            if let Some((start_offset, end_offset)) = selection_updates {
                let start_position = self.offset_to_position(start_offset);
                let end_position = self.offset_to_position(end_offset);
                
                if let Some(ref mut selection) = self.cursors[i].selection {
                    selection.start.offset = start_offset;
                    selection.end.offset = end_offset;
                    selection.start = start_position;
                    selection.end = end_position;
                }
            }
        }
    }

    fn push_undo_operation(&mut self, operation: TextOperation) {
        if self.undo_stack.len() >= self.max_undo_levels {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(operation);
        self.redo_stack.clear(); // Clear redo stack when new operation is performed
    }

    fn apply_undo_operation(&mut self, operation: &TextOperation) -> Result<(), TextProcessorError> {
        match operation {
            TextOperation::Insert { position, text, cursor_positions } => {
                self.content.drain(*position..*position + text.len());
                self.cursors = cursor_positions.clone();
                self.invalidate_cache();
            },
            TextOperation::Delete { position, text, cursor_positions } => {
                self.content.insert_str(*position, text);
                self.cursors = cursor_positions.clone();
                self.invalidate_cache();
            },
            TextOperation::Replace { start, end: _, old_text, new_text, cursor_positions } => {
                self.content.replace_range(*start..*start + new_text.len(), old_text);
                self.cursors = cursor_positions.clone();
                self.invalidate_cache();
            },
            TextOperation::FormatApply { cursor_positions, .. } |
            TextOperation::FormatRemove { cursor_positions, .. } => {
                self.cursors = cursor_positions.clone();
            },
        }
        Ok(())
    }

    fn apply_redo_operation(&mut self, operation: &TextOperation) -> Result<(), TextProcessorError> {
        match operation {
            TextOperation::Insert { position, text, .. } => {
                self.content.insert_str(*position, text);
                self.invalidate_cache();
            },
            TextOperation::Delete { position, text, .. } => {
                self.content.drain(*position..*position + text.len());
                self.invalidate_cache();
            },
            TextOperation::Replace { start, old_text, new_text, .. } => {
                self.content.replace_range(*start..*start + old_text.len(), new_text);
                self.invalidate_cache();
            },
            TextOperation::FormatApply { selections, format_type, .. } => {
                for selection in selections.iter().rev() {
                    self.apply_format_to_selection(selection, format_type)?;
                }
            },
            TextOperation::FormatRemove { .. } => {
                // Format removal would be implemented here
            },
        }
        Ok(())
    }
}

/// Text processor errors
#[derive(Debug, thiserror::Error)]
pub enum TextProcessorError {
    #[error("Invalid position: {0}")]
    InvalidPosition(usize),
    
    #[error("Invalid range: {0}-{1}")]
    InvalidRange(usize, usize),
    
    #[error("Regex error: {0}")]
    RegexError(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_processor_creation() {
        let processor = MarkdownTextProcessor::new();
        assert_eq!(processor.get_content(), "");
        assert_eq!(processor.get_cursors().len(), 1);
    }

    #[test]
    fn test_insert_text() {
        let mut processor = MarkdownTextProcessor::new();
        processor.insert_text(0, "Hello").unwrap();
        assert_eq!(processor.get_content(), "Hello");
    }

    #[test]
    fn test_delete_range() {
        let mut processor = MarkdownTextProcessor::with_content("Hello World".to_string());
        let deleted = processor.delete_range(5, 11).unwrap();
        assert_eq!(deleted, " World");
        assert_eq!(processor.get_content(), "Hello");
    }

    #[test]
    fn test_find_text() {
        let mut processor = MarkdownTextProcessor::with_content("Hello World Hello".to_string());
        let options = FindReplaceOptions::default();
        let matches = processor.find_text("Hello", &options).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start, 0);
        assert_eq!(matches[1].start, 12);
    }

    #[test]
    fn test_undo_redo() {
        let mut processor = MarkdownTextProcessor::new();
        processor.insert_text(0, "Hello").unwrap();
        assert_eq!(processor.get_content(), "Hello");
        
        processor.undo().unwrap();
        assert_eq!(processor.get_content(), "");
        
        processor.redo().unwrap();
        assert_eq!(processor.get_content(), "Hello");
    }

    #[test]
    fn test_apply_formatting() {
        let mut processor = MarkdownTextProcessor::with_content("Hello".to_string());
        let selection = TextSelection::new(
            TextPosition::new(0, 0, 0),
            TextPosition::new(0, 5, 5)
        );
        
        processor.cursors[0].selection = Some(selection);
        processor.apply_formatting(MarkdownFormat::Bold).unwrap();
        assert_eq!(processor.get_content(), "**Hello**");
    }

    #[test]
    fn test_position_conversion() {
        let mut processor = MarkdownTextProcessor::with_content("Line 1\nLine 2\nLine 3".to_string());
        let pos = processor.offset_to_position(7); // Start of "Line 2"
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 0);
        
        let offset = processor.position_to_offset(&TextPosition::new(1, 0, 7));
        assert_eq!(offset, 7);
    }
}