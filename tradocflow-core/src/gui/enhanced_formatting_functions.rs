use anyhow::Result;

/// Enhanced text selection information
#[derive(Debug, Clone)]
pub struct TextSelection {
    pub content: String,
    pub start: usize,
    pub end: usize,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

/// Result of a formatting operation
#[derive(Debug, Clone)]
pub struct FormattingResult {
    pub new_content: String,
    pub new_cursor_pos: usize,
    pub new_selection: Option<(usize, usize)>,
    pub status_message: String,
}

/// Enhanced markdown formatting functions with intelligent text manipulation
pub struct EnhancedFormattingEngine {
    /// Common sample texts for different formatting types
    pub sample_texts: SampleTexts,
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

impl EnhancedFormattingEngine {
    pub fn new() -> Self {
        Self {
            sample_texts: SampleTexts::default(),
        }
    }

    /// Parse current selection and content state
    pub fn parse_selection(&self, content: &str, cursor_pos: usize, selection_start: Option<usize>, selection_length: Option<usize>) -> TextSelection {
        let _lines: Vec<&str> = content.lines().collect();
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

    /// Get the current line content
    fn get_current_line(&self, content: &str, line: usize) -> (String, usize, usize) {
        let lines: Vec<&str> = content.lines().collect();
        if line >= lines.len() {
            return (String::new(), 0, 0);
        }
        
        let line_content = lines[line].to_string();
        let line_start = if line == 0 {
            0
        } else {
            content.lines().take(line).map(|l| l.len() + 1).sum::<usize>()
        };
        let line_end = line_start + line_content.len();
        
        (line_content, line_start, line_end)
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

    /// Enhanced italic formatting with smart text selection
    pub fn format_italic(&self, content: &str, selection: TextSelection) -> Result<FormattingResult> {
        if !selection.content.is_empty() {
            // Text is selected - wrap it with italic formatting
            let is_already_italic = selection.content.starts_with("*") && selection.content.ends_with("*") && !selection.content.starts_with("**") && selection.content.len() > 2;
            
            let (new_text, cursor_offset) = if is_already_italic {
                // Remove italic formatting
                let inner_text = &selection.content[1..selection.content.len()-1];
                (inner_text.to_string(), 0)
            } else {
                // Add italic formatting
                (format!("*{}*", selection.content), 1)
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
                status_message: if is_already_italic { "Removed italic formatting" } else { "Applied italic formatting" }.to_string(),
            })
        } else {
            // No text selected - insert sample italic text
            let sample_italic = format!("*{}*", self.sample_texts.italic);
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                sample_italic,
                &content[selection.start..]
            );
            
            let cursor_pos_inside = selection.start + 1; // Position cursor inside the *
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: cursor_pos_inside,
                new_selection: Some((cursor_pos_inside, cursor_pos_inside + self.sample_texts.italic.len())),
                status_message: "Inserted italic text template".to_string(),
            })
        }
    }

    /// Enhanced strikethrough formatting
    pub fn format_strikethrough(&self, content: &str, selection: TextSelection) -> Result<FormattingResult> {
        if !selection.content.is_empty() {
            // Text is selected - wrap it with strikethrough formatting
            let is_already_strikethrough = selection.content.starts_with("~~") && selection.content.ends_with("~~") && selection.content.len() > 4;
            
            let (new_text, cursor_offset) = if is_already_strikethrough {
                // Remove strikethrough formatting
                let inner_text = &selection.content[2..selection.content.len()-2];
                (inner_text.to_string(), 0)
            } else {
                // Add strikethrough formatting
                (format!("~~{}~~", selection.content), 2)
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
                status_message: if is_already_strikethrough { "Removed strikethrough formatting" } else { "Applied strikethrough formatting" }.to_string(),
            })
        } else {
            // No text selected - insert sample strikethrough text
            let sample_strikethrough = format!("~~{}~~", self.sample_texts.strikethrough);
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                sample_strikethrough,
                &content[selection.start..]
            );
            
            let cursor_pos_inside = selection.start + 2; // Position cursor inside the ~~
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: cursor_pos_inside,
                new_selection: Some((cursor_pos_inside, cursor_pos_inside + self.sample_texts.strikethrough.len())),
                status_message: "Inserted strikethrough text template".to_string(),
            })
        }
    }

    /// Enhanced inline code formatting
    pub fn format_inline_code(&self, content: &str, selection: TextSelection) -> Result<FormattingResult> {
        if !selection.content.is_empty() {
            // Text is selected - wrap it with code formatting
            let is_already_code = selection.content.starts_with("`") && selection.content.ends_with("`") && selection.content.len() > 2;
            
            let (new_text, cursor_offset) = if is_already_code {
                // Remove code formatting
                let inner_text = &selection.content[1..selection.content.len()-1];
                (inner_text.to_string(), 0)
            } else {
                // Add code formatting
                (format!("`{}`", selection.content), 1)
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
                status_message: if is_already_code { "Removed inline code formatting" } else { "Applied inline code formatting" }.to_string(),
            })
        } else {
            // No text selected - insert sample code text
            let sample_code = format!("`{}`", self.sample_texts.code);
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                sample_code,
                &content[selection.start..]
            );
            
            let cursor_pos_inside = selection.start + 1; // Position cursor inside the `
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: cursor_pos_inside,
                new_selection: Some((cursor_pos_inside, cursor_pos_inside + self.sample_texts.code.len())),
                status_message: "Inserted inline code template".to_string(),
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
            // No selection - check if cursor is on a line, convert that line to heading
            let (line_content, line_start, line_end) = self.get_current_line(content, selection.cursor_line);
            
            if !line_content.trim().is_empty() {
                // Line has content - convert it to heading
                let trimmed_content = line_content.trim_start_matches('#').trim();
                let new_line = format!("{} {}", heading_prefix, trimmed_content);
                
                let new_content = format!("{}{}{}", 
                    &content[..line_start], 
                    new_line,
                    &content[line_end..]
                );
                
                Ok(FormattingResult {
                    new_content,
                    new_cursor_pos: line_start + new_line.len(),
                    new_selection: None,
                    status_message: format!("Converted line to H{} heading", level),
                })
            } else {
                // Empty line - insert sample heading
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
    }

    /// Enhanced bullet list formatting with intelligent list management
    pub fn format_bullet_list(&self, content: &str, selection: TextSelection) -> Result<FormattingResult> {
        if !selection.content.is_empty() {
            // Text is selected - convert to bullet list items
            let lines: Vec<&str> = selection.content.lines().collect();
            let formatted_lines: Vec<String> = lines.into_iter().map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    String::new()
                } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    // Already a list item, keep as is
                    line.to_string()
                } else {
                    format!("- {}", trimmed)
                }
            }).collect();
            
            let new_text = formatted_lines.join("\n");
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_text,
                &content[selection.end..]
            );
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: selection.start + new_text.len(),
                new_selection: None,
                status_message: "Converted to bullet list".to_string(),
            })
        } else {
            // No selection - insert new bullet list items
            let new_items = format!("- {}\n- {}\n- {}", 
                self.sample_texts.bullet_item, 
                self.sample_texts.bullet_item, 
                self.sample_texts.bullet_item
            );
            
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_items,
                &content[selection.start..]
            );
            
            let cursor_pos = selection.start + 2; // Position after "- "
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: cursor_pos,
                new_selection: Some((cursor_pos, cursor_pos + self.sample_texts.bullet_item.len())),
                status_message: "Inserted bullet list template".to_string(),
            })
        }
    }

    /// Enhanced numbered list formatting
    pub fn format_numbered_list(&self, content: &str, selection: TextSelection) -> Result<FormattingResult> {
        if !selection.content.is_empty() {
            // Text is selected - convert to numbered list items
            let lines: Vec<&str> = selection.content.lines().collect();
            let formatted_lines: Vec<String> = lines.into_iter().enumerate().map(|(i, line)| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    String::new()
                } else {
                    // Remove existing list formatting
                    let clean_text = if let Some(pos) = trimmed.find(". ") {
                        if trimmed.chars().take(pos).all(|c| c.is_ascii_digit()) {
                            &trimmed[pos + 2..]
                        } else {
                            trimmed
                        }
                    } else {
                        trimmed.trim_start_matches("- ").trim_start_matches("* ")
                    };
                    format!("{}. {}", i + 1, clean_text)
                }
            }).collect();
            
            let new_text = formatted_lines.join("\n");
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_text,
                &content[selection.end..]
            );
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: selection.start + new_text.len(),
                new_selection: None,
                status_message: "Converted to numbered list".to_string(),
            })
        } else {
            // No selection - insert new numbered list items
            let new_items = format!("1. {}\n2. {}\n3. {}", 
                self.sample_texts.numbered_item, 
                self.sample_texts.numbered_item, 
                self.sample_texts.numbered_item
            );
            
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_items,
                &content[selection.start..]
            );
            
            let cursor_pos = selection.start + 3; // Position after "1. "
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: cursor_pos,
                new_selection: Some((cursor_pos, cursor_pos + self.sample_texts.numbered_item.len())),
                status_message: "Inserted numbered list template".to_string(),
            })
        }
    }

    /// Task list formatting
    pub fn format_task_list(&self, content: &str, selection: TextSelection) -> Result<FormattingResult> {
        if !selection.content.is_empty() {
            // Text is selected - convert to task list items
            let lines: Vec<&str> = selection.content.lines().collect();
            let formatted_lines: Vec<String> = lines.into_iter().map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    String::new()
                } else if trimmed.starts_with("- [ ] ") || trimmed.starts_with("- [x] ") {
                    // Already a task item, keep as is
                    line.to_string()
                } else {
                    // Remove other list formatting and add task formatting
                    let clean_text = trimmed.trim_start_matches("- ").trim_start_matches("* ");
                    let clean_text = if let Some(pos) = clean_text.find(". ") {
                        if clean_text.chars().take(pos).all(|c| c.is_ascii_digit()) {
                            &clean_text[pos + 2..]
                        } else {
                            clean_text
                        }
                    } else {
                        clean_text
                    };
                    format!("- [ ] {}", clean_text)
                }
            }).collect();
            
            let new_text = formatted_lines.join("\n");
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_text,
                &content[selection.end..]
            );
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: selection.start + new_text.len(),
                new_selection: None,
                status_message: "Converted to task list".to_string(),
            })
        } else {
            // No selection - insert new task list items
            let new_items = format!("- [x] {}\n- [ ] {}\n- [ ] {}", 
                "Completed task",
                self.sample_texts.task_item, 
                self.sample_texts.task_item
            );
            
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_items,
                &content[selection.start..]
            );
            
            let cursor_pos = selection.start + "- [x] ".len(); // Position after first task checkbox
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: cursor_pos,
                new_selection: Some((cursor_pos, cursor_pos + "Completed task".len())),
                status_message: "Inserted task list template".to_string(),
            })
        }
    }

    /// Block quote formatting
    pub fn format_blockquote(&self, content: &str, selection: TextSelection) -> Result<FormattingResult> {
        if !selection.content.is_empty() {
            // Text is selected - convert to blockquote
            let lines: Vec<&str> = selection.content.lines().collect();
            let formatted_lines: Vec<String> = lines.into_iter().map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    ">".to_string()
                } else if trimmed.starts_with("> ") {
                    // Already a blockquote, keep as is
                    line.to_string()
                } else {
                    format!("> {}", trimmed)
                }
            }).collect();
            
            let new_text = formatted_lines.join("\n");
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_text,
                &content[selection.end..]
            );
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: selection.start + new_text.len(),
                new_selection: None,
                status_message: "Converted to blockquote".to_string(),
            })
        } else {
            // No selection - insert blockquote template
            let sample_quote = format!("> {}", self.sample_texts.blockquote_text);
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                sample_quote,
                &content[selection.start..]
            );
            
            let cursor_pos = selection.start + 2; // Position after "> "
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: cursor_pos,
                new_selection: Some((cursor_pos, cursor_pos + self.sample_texts.blockquote_text.len())),
                status_message: "Inserted blockquote template".to_string(),
            })
        }
    }

    /// Code block formatting
    pub fn format_code_block(&self, content: &str, selection: TextSelection, language: Option<&str>) -> Result<FormattingResult> {
        let lang_spec = language.unwrap_or("");
        
        if !selection.content.is_empty() {
            // Text is selected - wrap in code block
            let new_text = format!("```{}\n{}\n```", lang_spec, selection.content);
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                new_text,
                &content[selection.end..]
            );
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: selection.start + new_text.len(),
                new_selection: None,
                status_message: if lang_spec.is_empty() { 
                    "Wrapped in code block".to_string() 
                } else { 
                    format!("Wrapped in {} code block", lang_spec) 
                },
            })
        } else {
            // No selection - insert code block template
            let sample_block = format!("```{}\n{}\n```", lang_spec, self.sample_texts.code_block_sample);
            let new_content = format!("{}{}{}", 
                &content[..selection.start], 
                sample_block,
                &content[selection.start..]
            );
            
            let cursor_pos = selection.start + format!("```{}\n", lang_spec).len(); // Position inside code block
            
            Ok(FormattingResult {
                new_content,
                new_cursor_pos: cursor_pos,
                new_selection: Some((cursor_pos, cursor_pos + self.sample_texts.code_block_sample.len())),
                status_message: "Inserted code block template".to_string(),
            })
        }
    }

    /// Link creation with proper structure
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

    /// Image insertion with alt text
    pub fn create_image(&self, content: &str, selection: TextSelection, url: Option<&str>) -> Result<FormattingResult> {
        let image_url = url.unwrap_or(self.sample_texts.image_url);
        
        let alt_text = if !selection.content.is_empty() {
            selection.content.clone()
        } else {
            self.sample_texts.image_alt.to_string()
        };
        
        let new_text = format!("![{}]({})", alt_text, image_url);
        let new_content = format!("{}{}{}", 
            &content[..selection.start], 
            new_text,
            &content[selection.end..]
        );
        
        let alt_start = selection.start + 2; // Position inside alt text
        let alt_end = alt_start + alt_text.len();
        
        Ok(FormattingResult {
            new_content,
            new_cursor_pos: alt_start,
            new_selection: Some((alt_start, alt_end)),
            status_message: "Inserted image".to_string(),
        })
    }

    /// Table creation
    pub fn create_table(&self, content: &str, selection: TextSelection, rows: Option<usize>, cols: Option<usize>) -> Result<FormattingResult> {
        let num_rows = rows.unwrap_or(2); // Header + 1 data row by default
        let num_cols = cols.unwrap_or(3);
        
        // Create header row
        let headers: Vec<String> = (0..num_cols).map(|i| {
            if i < self.sample_texts.table_headers.len() {
                self.sample_texts.table_headers[i].to_string()
            } else {
                format!("Header {}", i + 1)
            }
        }).collect();
        
        // Create separator row
        let separator: Vec<String> = (0..num_cols).map(|_| "--------".to_string()).collect();
        
        // Create data rows
        let mut all_rows = vec![
            format!("| {} |", headers.join(" | ")),
            format!("| {} |", separator.join(" | ")),
        ];
        
        for _ in 1..num_rows {
            let row_data: Vec<String> = (0..num_cols).map(|i| {
                if i < self.sample_texts.table_row.len() {
                    self.sample_texts.table_row[i].to_string()
                } else {
                    format!("Cell {}", i + 1)
                }
            }).collect();
            all_rows.push(format!("| {} |", row_data.join(" | ")));
        }
        
        let new_table = all_rows.join("\n");
        let new_content = format!("{}{}{}", 
            &content[..selection.start], 
            new_table,
            &content[selection.end..]
        );
        
        // Position cursor in first cell of first data row
        let first_cell_pos = selection.start + all_rows[0].len() + all_rows[1].len() + 4; // After header and separator + "| "
        
        Ok(FormattingResult {
            new_content,
            new_cursor_pos: first_cell_pos,
            new_selection: Some((first_cell_pos, first_cell_pos + self.sample_texts.table_row[0].len())),
            status_message: format!("Inserted {}×{} table", num_rows, num_cols),
        })
    }
}

impl Default for EnhancedFormattingEngine {
    fn default() -> Self {
        Self::new()
    }
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
    fn test_heading_formatting_empty_selection() {
        let engine = EnhancedFormattingEngine::new();
        let content = "Some text\n\nMore text";
        let selection = TextSelection {
            content: String::new(),
            start: 10, // At empty line
            end: 10,
            cursor_line: 1,
            cursor_col: 0,
        };
        
        let result = engine.format_heading(content, selection, 2).unwrap();
        assert!(result.new_content.contains("## Heading 2"));
        assert!(result.status_message.contains("template"));
    }

    #[test]
    fn test_list_formatting() {
        let engine = EnhancedFormattingEngine::new();
        let content = "Item one\nItem two";
        let selection = TextSelection {
            content: "Item one\nItem two".to_string(),
            start: 0,
            end: 17,
            cursor_line: 0,
            cursor_col: 0,
        };
        
        let result = engine.format_bullet_list(content, selection).unwrap();
        assert!(result.new_content.contains("- Item one"));
        assert!(result.new_content.contains("- Item two"));
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
}