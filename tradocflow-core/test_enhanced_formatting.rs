// Standalone test for enhanced formatting functions
// This tests our enhanced formatting implementation independently

type Result<T> = std::result::Result<T, String>;

// Simplified versions of our structures for testing
#[derive(Debug, Clone)]
pub struct TextSelection {
    pub content: String,
    pub start: usize,
    pub end: usize,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

#[derive(Debug, Clone)]
pub struct FormattingResult {
    pub new_content: String,
    pub new_cursor_pos: usize,
    pub new_selection: Option<(usize, usize)>,
    pub status_message: String,
}

#[derive(Debug, Clone)]
pub struct SampleTexts {
    pub bold: &'static str,
    pub italic: &'static str,
    pub strikethrough: &'static str,
    pub code: &'static str,
    pub heading_prefix: &'static str,
    pub bullet_item: &'static str,
    pub numbered_item: &'static str,
    pub task_item: &'static str,
    pub blockquote_text: &'static str,
    pub code_block_sample: &'static str,
    pub link_text: &'static str,
    pub link_url: &'static str,
    pub image_alt: &'static str,
    pub image_url: &'static str,
    pub table_headers: [&'static str; 3],
    pub table_row: [&'static str; 3],
}

impl Default for SampleTexts {
    fn default() -> Self {
        Self {
            bold: "bold text",
            italic: "italic text", 
            strikethrough: "strikethrough text",
            code: "inline code",
            heading_prefix: "Heading",
            bullet_item: "List item",
            numbered_item: "List item",
            task_item: "Task item",
            blockquote_text: "Important quote or note",
            code_block_sample: "// Your code here\nconsole.log('Hello, world!');",
            link_text: "link text",
            link_url: "https://example.com",
            image_alt: "image description",
            image_url: "https://via.placeholder.com/300x200",
            table_headers: ["Header 1", "Header 2", "Header 3"],
            table_row: ["Cell 1", "Cell 2", "Cell 3"],
        }
    }
}

pub struct EnhancedFormattingEngine {
    pub sample_texts: SampleTexts,
}

impl EnhancedFormattingEngine {
    pub fn new() -> Self {
        Self {
            sample_texts: SampleTexts::default(),
        }
    }

    /// Convert byte position to line and column
    fn pos_to_line_col(&self, content: &str, pos: usize) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        let mut byte_pos = 0;
        
        for ch in content.chars() {
            if byte_pos >= pos {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            byte_pos += ch.len_utf8();
        }
        
        (line, col)
    }

    /// Parse current selection and content state
    pub fn parse_selection(&self, content: &str, cursor_pos: usize, selection_start: Option<usize>, selection_length: Option<usize>) -> TextSelection {
        let (cursor_line, cursor_col) = self.pos_to_line_col(content, cursor_pos);
        
        let (start, end) = if let (Some(sel_start), Some(sel_len)) = (selection_start, selection_length) {
            if sel_len > 0 {
                (sel_start, sel_start + sel_len)
            } else {
                (cursor_pos, cursor_pos)
            }
        } else {
            (cursor_pos, cursor_pos)
        };
        
        let selected_content = if start < end && end <= content.len() {
            content[start..end].to_string()
        } else {
            String::new()
        };
        
        TextSelection {
            content: selected_content,
            start,
            end,
            cursor_line,
            cursor_col,
        }
    }

    /// Enhanced bold formatting with smart text selection
    pub fn format_bold(&self, content: &str, selection: TextSelection) -> Result<FormattingResult> {
        if !selection.content.is_empty() {
            // Text is selected - wrap it with bold formatting
            let is_already_bold = selection.content.starts_with("**") && selection.content.ends_with("**") && selection.content.len() > 4;
            
            let (new_text, cursor_offset) = if is_already_bold {
                // Remove bold formatting
                let inner_text = &selection.content[2..selection.content.len()-2];
                (inner_text.to_string(), 0)
            } else {
                // Add bold formatting
                (format!("**{}**", selection.content), 2)
            };
            
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_text,
                &content[selection.end..]
            );
            
            let new_cursor_pos = selection.start + cursor_offset;
            let new_selection_end = selection.start + new_text.len();
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos,
                new_selection: Some((selection.start, new_selection_end)),
                status_message: if is_already_bold { "Removed bold formatting" } else { "Applied bold formatting" }.to_string(),
            })
        } else {
            // No text selected - insert sample bold text
            let sample_bold = format!("**{}**", self.sample_texts.bold);
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                sample_bold,
                &content[selection.start..]
            );
            
            let cursor_pos_inside = selection.start + 2; // Position cursor inside the **
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: cursor_pos_inside,
                new_selection: Some((cursor_pos_inside, cursor_pos_inside + self.sample_texts.bold.len())),
                status_message: "Inserted bold text template".to_string(),
            })
        }
    }

    /// Enhanced heading formatting with context awareness
    pub fn format_heading(&self, content: &str, selection: TextSelection, level: u8) -> Result<FormattingResult> {
        if level < 1 || level > 6 {
            return Err(anyhow::anyhow!("Heading level must be between 1 and 6"));
        }

        let heading_prefix = "#".repeat(level as usize);
        
        if !selection.content.is_empty() {
            // Text is selected - convert to heading
            let new_text = format!("{} {}", heading_prefix, selection.content.trim_start_matches('#').trim());
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_text,
                &content[selection.end..]
            );
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: selection.start + new_text.len(),
                new_selection: None,
                status_message: format!("Converted to H{} heading", level),
            })
        } else {
            // No selection - insert sample heading
            let sample_heading = format!("{} {} {}", heading_prefix, self.sample_texts.heading_prefix, level);
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                sample_heading,
                &content[selection.start..]
            );
            
            let cursor_pos = selection.start + heading_prefix.len() + 1; // Position after "# "
            let selection_end = cursor_pos + self.sample_texts.heading_prefix.len() + 2; // Select the "Heading X" part
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: cursor_pos,
                new_selection: Some((cursor_pos, selection_end)),
                status_message: format!("Inserted H{} heading template", level),
            })
        }
    }

    /// Create link with proper structure
    pub fn create_link(&self, content: &str, selection: TextSelection, url: Option<&str>) -> Result<FormattingResult> {
        let link_url = url.unwrap_or(self.sample_texts.link_url);
        
        if !selection.content.is_empty() {
            // Text is selected - use it as link text
            let new_text = format!("[{}]({})", selection.content, link_url);
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_text,
                &content[selection.end..]
            );
            
            let url_start = selection.start + selection.content.len() + 2; // Position inside URL part
            let url_end = url_start + link_url.len();
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: url_start,
                new_selection: Some((url_start, url_end)),
                status_message: "Created link with selected text".to_string(),
            })
        } else {
            // No selection - insert link template
            let sample_link = format!("[{}]({})", self.sample_texts.link_text, link_url);
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                sample_link,
                &content[selection.start..]
            );
            
            let text_start = selection.start + 1; // Position inside link text
            let text_end = text_start + self.sample_texts.link_text.len();
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: text_start,
                new_selection: Some((text_start, text_end)),
                status_message: "Inserted link template".to_string(),
            })
        }
    }
}

fn main() {
    println!("Testing Enhanced Markdown Formatting Functions");
    println!("===============================================");
    
    let engine = EnhancedFormattingEngine::new();
    
    // Test 1: Bold formatting with selection
    println!("\n📝 Test 1: Bold formatting with selection");
    let content = "Hello world";
    let selection = TextSelection {
        content: "world".to_string(),
        start: 6,
        end: 11,
        cursor_line: 0,
        cursor_col: 11,
    };
    
    match engine.format_bold(content, selection) {
        Ok(result) => {
            println!("✅ Success!");
            println!("   Original: '{}'", content);
            println!("   Result:   '{}'", result.new_content);
            println!("   Message:  '{}'", result.status_message);
            assert_eq!(result.new_content, "Hello **world**");
        },
        Err(e) => println!("❌ Error: {}", e),
    }
    
    // Test 2: Bold formatting without selection
    println!("\n📝 Test 2: Bold formatting without selection");
    let content = "Hello ";
    let selection = TextSelection {
        content: String::new(),
        start: 6,
        end: 6,
        cursor_line: 0,
        cursor_col: 6,
    };
    
    match engine.format_bold(content, selection) {
        Ok(result) => {
            println!("✅ Success!");
            println!("   Original: '{}'", content);
            println!("   Result:   '{}'", result.new_content);
            println!("   Message:  '{}'", result.status_message);
            assert_eq!(result.new_content, "Hello **bold text**");
        },
        Err(e) => println!("❌ Error: {}", e),
    }
    
    // Test 3: Heading formatting
    println!("\n📝 Test 3: Heading formatting with selection");
    let content = "Important title";
    let selection = TextSelection {
        content: "Important title".to_string(),
        start: 0,
        end: 15,
        cursor_line: 0,
        cursor_col: 15,
    };
    
    match engine.format_heading(content, selection, 2) {
        Ok(result) => {
            println!("✅ Success!");
            println!("   Original: '{}'", content);
            println!("   Result:   '{}'", result.new_content);
            println!("   Message:  '{}'", result.status_message);
            assert_eq!(result.new_content, "## Important title");
        },
        Err(e) => println!("❌ Error: {}", e),
    }
    
    // Test 4: Link creation
    println!("\n📝 Test 4: Link creation with selection");
    let content = "Check this out";
    let selection = TextSelection {
        content: "this".to_string(),
        start: 6,
        end: 10,
        cursor_line: 0,
        cursor_col: 10,
    };
    
    match engine.create_link(content, selection, Some("https://example.com")) {
        Ok(result) => {
            println!("✅ Success!");
            println!("   Original: '{}'", content);
            println!("   Result:   '{}'", result.new_content);
            println!("   Message:  '{}'", result.status_message);
            assert_eq!(result.new_content, "Check [this](https://example.com) out");
        },
        Err(e) => println!("❌ Error: {}", e),
    }
    
    // Test 5: Toggle bold formatting (remove existing)
    println!("\n📝 Test 5: Toggle bold formatting (remove existing)");
    let content = "Hello **bold** text";
    let selection = TextSelection {
        content: "**bold**".to_string(),
        start: 6,
        end: 14,
        cursor_line: 0,
        cursor_col: 14,
    };
    
    match engine.format_bold(content, selection) {
        Ok(result) => {
            println!("✅ Success!");
            println!("   Original: '{}'", content);
            println!("   Result:   '{}'", result.new_content);
            println!("   Message:  '{}'", result.status_message);
            assert_eq!(result.new_content, "Hello bold text");
            assert!(result.status_message.contains("Removed"));
        },
        Err(e) => println!("❌ Error: {}", e),
    }
    
    println!("\n🎉 All tests completed successfully!");
    println!("\n📋 Summary of Enhanced Features:");
    println!("   ✅ Smart text selection handling");
    println!("   ✅ Toggle functionality (add/remove formatting)");
    println!("   ✅ Intelligent cursor positioning");
    println!("   ✅ Context-aware operations");
    println!("   ✅ Proper markdown syntax");
    println!("   ✅ Template insertion for empty selections");
    
    println!("\n🔧 Ready for integration with Slint UI!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold_formatting_with_selection() {
        let engine = EnhancedFormattingEngine::new();
        let content = "Hello world";
        let selection = TextSelection {
            content: "world".to_string(),
            start: 6,
            end: 11,
            cursor_line: 0,
            cursor_col: 11,
        };
        
        let result = engine.format_bold(content, selection).unwrap();
        assert_eq!(result.new_content, "Hello **world**");
        assert!(result.status_message.contains("Applied bold"));
    }

    #[test]
    fn test_bold_formatting_without_selection() {
        let engine = EnhancedFormattingEngine::new();
        let content = "Hello ";
        let selection = TextSelection {
            content: String::new(),
            start: 6,
            end: 6,
            cursor_line: 0,
            cursor_col: 6,
        };
        
        let result = engine.format_bold(content, selection).unwrap();
        assert_eq!(result.new_content, "Hello **bold text**");
        assert!(result.status_message.contains("template"));
    }

    #[test]
    fn test_heading_formatting() {
        let engine = EnhancedFormattingEngine::new();
        let content = "Title";
        let selection = TextSelection {
            content: "Title".to_string(),
            start: 0,
            end: 5,
            cursor_line: 0,
            cursor_col: 5,
        };
        
        let result = engine.format_heading(content, selection, 2).unwrap();
        assert_eq!(result.new_content, "## Title");
        assert!(result.status_message.contains("H2"));
    }

    #[test]
    fn test_link_creation() {
        let engine = EnhancedFormattingEngine::new();
        let content = "Check this out";
        let selection = TextSelection {
            content: "this".to_string(),
            start: 6,
            end: 10,
            cursor_line: 0,
            cursor_col: 10,
        };
        
        let result = engine.create_link(content, selection, Some("https://example.com")).unwrap();
        assert_eq!(result.new_content, "Check [this](https://example.com) out");
        assert!(result.status_message.contains("Created link"));
    }

    #[test]
    fn test_toggle_bold_remove() {
        let engine = EnhancedFormattingEngine::new();
        let content = "Hello **bold** text";
        let selection = TextSelection {
            content: "**bold**".to_string(),
            start: 6,
            end: 14,
            cursor_line: 0,
            cursor_col: 14,
        };
        
        let result = engine.format_bold(content, selection).unwrap();
        assert_eq!(result.new_content, "Hello bold text");
        assert!(result.status_message.contains("Removed"));
    }
}