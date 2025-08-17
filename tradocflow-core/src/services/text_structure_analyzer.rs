use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use regex::Regex;
use uuid::Uuid;
use crate::Result;
use crate::services::sentence_alignment_service::{LanguageProfile, SentenceBoundary};

/// Different types of text structures that can be analyzed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TextStructureType {
    Paragraph,
    Heading { level: u8 },
    List { list_type: ListType },
    CodeBlock { language: Option<String> },
    Table,
    Quote,
    HorizontalRule,
    LinkReference,
    ImageReference,
    Custom(String),
}

/// Type of list structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListType {
    Ordered,
    Unordered,
    Definition,
    Task,
}

/// A structural element detected in text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStructure {
    pub id: Uuid,
    pub structure_type: TextStructureType,
    pub start_offset: usize,
    pub end_offset: usize,
    pub content: String,
    pub nesting_level: u8,
    pub parent_id: Option<Uuid>,
    pub children: Vec<Uuid>,
    pub metadata: HashMap<String, String>,
}

/// Analysis result for text structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureAnalysisResult {
    pub structures: Vec<TextStructure>,
    pub hierarchy: StructureHierarchy,
    pub statistics: StructureStatistics,
    pub language_specific_features: LanguageSpecificFeatures,
}

/// Hierarchical representation of text structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureHierarchy {
    pub root_elements: Vec<Uuid>,
    pub max_depth: u8,
    pub structure_tree: HashMap<Uuid, Vec<Uuid>>, // parent -> children mapping
}

/// Statistics about text structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureStatistics {
    pub total_elements: usize,
    pub element_counts: HashMap<String, usize>, // structure type -> count
    pub average_paragraph_length: f64,
    pub sentence_count: usize,
    pub word_count: usize,
    pub character_count: usize,
    pub complexity_score: f64,
}

/// Language-specific structural features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageSpecificFeatures {
    pub detected_language: String,
    pub confidence: f64,
    pub writing_direction: WritingDirection,
    pub special_characters: Vec<SpecialCharacter>,
    pub formatting_patterns: Vec<FormattingPattern>,
}

/// Writing direction for different languages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WritingDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    Mixed,
}

/// Special characters detected in text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialCharacter {
    pub character: char,
    pub count: usize,
    pub positions: Vec<usize>,
    pub context_type: CharacterContext,
}

/// Context where special characters appear
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CharacterContext {
    Punctuation,
    Quote,
    Bullet,
    Mathematical,
    Currency,
    Diacritic,
    Other,
}

/// Formatting patterns found in text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingPattern {
    pub pattern_type: PatternType,
    pub occurrences: Vec<PatternOccurrence>,
    pub confidence: f64,
}

/// Types of formatting patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternType {
    Bold,
    Italic,
    Code,
    Link,
    Reference,
    Emphasis,
    Strikethrough,
    Underline,
}

/// Occurrence of a formatting pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternOccurrence {
    pub start_offset: usize,
    pub end_offset: usize,
    pub text: String,
    pub syntax_used: String, // e.g., "**text**", "*text*", "`code`"
}

/// Configuration for text structure analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureAnalysisConfig {
    pub detect_headings: bool,
    pub detect_lists: bool,
    pub detect_code_blocks: bool,
    pub detect_tables: bool,
    pub detect_quotes: bool,
    pub analyze_language_features: bool,
    pub max_nesting_depth: u8,
    pub minimum_paragraph_length: usize,
    pub detect_formatting_patterns: bool,
}

impl Default for StructureAnalysisConfig {
    fn default() -> Self {
        Self {
            detect_headings: true,
            detect_lists: true,
            detect_code_blocks: true,
            detect_tables: true,
            detect_quotes: true,
            analyze_language_features: true,
            max_nesting_depth: 10,
            minimum_paragraph_length: 10,
            detect_formatting_patterns: true,
        }
    }
}

/// Service for analyzing text structure and supporting sentence alignment
pub struct TextStructureAnalyzer {
    config: StructureAnalysisConfig,
    language_patterns: HashMap<String, LanguagePatterns>,
}

/// Language-specific patterns for structure detection
#[derive(Debug, Clone)]
struct LanguagePatterns {
    quote_patterns: Vec<Regex>,
    list_patterns: Vec<Regex>,
    emphasis_patterns: Vec<Regex>,
    special_char_frequencies: HashMap<char, f64>,
}

impl TextStructureAnalyzer {
    /// Create a new text structure analyzer
    pub fn new(config: StructureAnalysisConfig) -> Result<Self> {
        let mut language_patterns = HashMap::new();
        
        // Initialize patterns for common languages
        language_patterns.insert("en".to_string(), Self::create_english_patterns()?);
        language_patterns.insert("es".to_string(), Self::create_spanish_patterns()?);
        language_patterns.insert("fr".to_string(), Self::create_french_patterns()?);
        language_patterns.insert("de".to_string(), Self::create_german_patterns()?);

        Ok(Self {
            config,
            language_patterns,
        })
    }

    /// Analyze the structure of text
    pub async fn analyze_structure(
        &self,
        text: &str,
        language_hint: Option<&str>,
    ) -> Result<StructureAnalysisResult> {
        // Detect language if not provided
        let detected_language = if let Some(lang) = language_hint {
            lang.to_string()
        } else {
            self.detect_language(text).await?
        };

        // Analyze different structural elements
        let mut structures = Vec::new();
        let mut element_id_counter = 0u32;

        // Detect headings
        if self.config.detect_headings {
            let headings = self.detect_headings(text, &mut element_id_counter)?;
            structures.extend(headings);
        }

        // Detect lists
        if self.config.detect_lists {
            let lists = self.detect_lists(text, &mut element_id_counter)?;
            structures.extend(lists);
        }

        // Detect code blocks
        if self.config.detect_code_blocks {
            let code_blocks = self.detect_code_blocks(text, &mut element_id_counter)?;
            structures.extend(code_blocks);
        }

        // Detect tables
        if self.config.detect_tables {
            let tables = self.detect_tables(text, &mut element_id_counter)?;
            structures.extend(tables);
        }

        // Detect quotes
        if self.config.detect_quotes {
            let quotes = self.detect_quotes(text, &detected_language, &mut element_id_counter)?;
            structures.extend(quotes);
        }

        // Detect paragraphs (fill in remaining text)
        let paragraphs = self.detect_paragraphs(text, &structures, &mut element_id_counter)?;
        structures.extend(paragraphs);

        // Sort structures by position
        structures.sort_by_key(|s| s.start_offset);

        // Build hierarchy
        let hierarchy = self.build_hierarchy(&structures);

        // Calculate statistics
        let statistics = self.calculate_statistics(text, &structures);

        // Analyze language-specific features
        let language_specific_features = if self.config.analyze_language_features {
            self.analyze_language_features(text, &detected_language).await?
        } else {
            LanguageSpecificFeatures {
                detected_language: detected_language.clone(),
                confidence: 0.5,
                writing_direction: WritingDirection::LeftToRight,
                special_characters: Vec::new(),
                formatting_patterns: Vec::new(),
            }
        };

        Ok(StructureAnalysisResult {
            structures,
            hierarchy,
            statistics,
            language_specific_features,
        })
    }

    /// Detect language of text
    async fn detect_language(&self, text: &str) -> Result<String> {
        // Simplified language detection based on character patterns
        let mut language_scores = HashMap::new();
        
        // Count character frequencies for each language
        for (language, patterns) in &self.language_patterns {
            let mut score = 0.0;
            
            for ch in text.chars() {
                if let Some(&frequency) = patterns.special_char_frequencies.get(&ch) {
                    score += frequency;
                }
            }
            
            // Normalize by text length
            if !text.is_empty() {
                score /= text.len() as f64;
            }
            
            language_scores.insert(language.clone(), score);
        }

        // Find the language with highest score
        let detected_language = language_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(lang, _)| lang.clone())
            .unwrap_or_else(|| "en".to_string());

        Ok(detected_language)
    }

    /// Detect heading structures
    fn detect_headings(&self, text: &str, id_counter: &mut u32) -> Result<Vec<TextStructure>> {
        let mut headings = Vec::new();
        
        // ATX-style headings (# ## ### etc.)
        let atx_regex = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
        
        // Setext-style headings (underlined with = or -)
        let setext_regex = Regex::new(r"^(.+)\n(=+|-+)$").unwrap();

        for line in text.lines() {
            let line_start = text.find(line).unwrap_or(0);
            
            if let Some(captures) = atx_regex.captures(line) {
                let level = captures.get(1).unwrap().as_str().len() as u8;
                let title = captures.get(2).unwrap().as_str().trim();
                
                headings.push(TextStructure {
                    id: Uuid::new_v4(),
                    structure_type: TextStructureType::Heading { level },
                    start_offset: line_start,
                    end_offset: line_start + line.len(),
                    content: title.to_string(),
                    nesting_level: level,
                    parent_id: None,
                    children: Vec::new(),
                    metadata: HashMap::new(),
                });
                
                *id_counter += 1;
            }
        }

        // Handle setext-style headings
        let text_with_newlines = format!("{}\n", text);
        for captures in setext_regex.captures_iter(&text_with_newlines) {
            let title = captures.get(1).unwrap().as_str().trim();
            let underline = captures.get(2).unwrap().as_str();
            let level = if underline.starts_with('=') { 1 } else { 2 };
            
            let title_start = text.find(title).unwrap_or(0);
            let underline_end = text.find(underline).unwrap_or(title_start) + underline.len();
            
            headings.push(TextStructure {
                id: Uuid::new_v4(),
                structure_type: TextStructureType::Heading { level },
                start_offset: title_start,
                end_offset: underline_end,
                content: title.to_string(),
                nesting_level: level,
                parent_id: None,
                children: Vec::new(),
                metadata: HashMap::new(),
            });
            
            *id_counter += 1;
        }

        Ok(headings)
    }

    /// Detect list structures
    fn detect_lists(&self, text: &str, id_counter: &mut u32) -> Result<Vec<TextStructure>> {
        let mut lists = Vec::new();
        
        // Unordered list patterns
        let unordered_regex = Regex::new(r"^(\s*)([-*+])\s+(.+)$").unwrap();
        
        // Ordered list patterns
        let ordered_regex = Regex::new(r"^(\s*)(\d+\.)\s+(.+)$").unwrap();
        
        // Task list patterns
        let task_regex = Regex::new(r"^(\s*)([-*+])\s+\[([ xX])\]\s+(.+)$").unwrap();

        let mut current_list: Option<(TextStructureType, usize, usize, Vec<String>)> = None;

        for (line_num, line) in text.lines().enumerate() {
            let line_start = text.lines().take(line_num).map(|l| l.len() + 1).sum::<usize>();
            
            let mut matched = false;

            // Check for task list items
            if let Some(captures) = task_regex.captures(line) {
                let indent = captures.get(1).unwrap().as_str().len();
                let item_text = captures.get(4).unwrap().as_str();
                
                if let Some((ref list_type, _start_pos, _, ref mut items)) = current_list {
                    if matches!(list_type, TextStructureType::List { list_type: ListType::Task }) {
                        items.push(format!("{}[{}] {}", " ".repeat(indent), 
                                          captures.get(3).unwrap().as_str(), item_text));
                        matched = true;
                    }
                }
                
                if !matched {
                    // Start new task list
                    current_list = Some((
                        TextStructureType::List { list_type: ListType::Task },
                        line_start,
                        line_start + line.len(),
                        vec![format!("{}[{}] {}", " ".repeat(indent), 
                                   captures.get(3).unwrap().as_str(), item_text)]
                    ));
                    matched = true;
                }
            }
            // Check for unordered list items
            else if let Some(captures) = unordered_regex.captures(line) {
                let indent = captures.get(1).unwrap().as_str().len();
                let item_text = captures.get(3).unwrap().as_str();
                
                if let Some((ref list_type, _start_pos, _, ref mut items)) = current_list {
                    if matches!(list_type, TextStructureType::List { list_type: ListType::Unordered }) {
                        items.push(format!("{}- {}", " ".repeat(indent), item_text));
                        matched = true;
                    }
                }
                
                if !matched {
                    // Start new unordered list
                    current_list = Some((
                        TextStructureType::List { list_type: ListType::Unordered },
                        line_start,
                        line_start + line.len(),
                        vec![format!("{}- {}", " ".repeat(indent), item_text)]
                    ));
                    matched = true;
                }
            }
            // Check for ordered list items
            else if let Some(captures) = ordered_regex.captures(line) {
                let indent = captures.get(1).unwrap().as_str().len();
                let item_text = captures.get(3).unwrap().as_str();
                
                if let Some((ref list_type, _start_pos, _, ref mut items)) = current_list {
                    if matches!(list_type, TextStructureType::List { list_type: ListType::Ordered }) {
                        items.push(format!("{}{}. {}", " ".repeat(indent), 
                                          captures.get(2).unwrap().as_str(), item_text));
                        matched = true;
                    }
                }
                
                if !matched {
                    // Start new ordered list
                    current_list = Some((
                        TextStructureType::List { list_type: ListType::Ordered },
                        line_start,
                        line_start + line.len(),
                        vec![format!("{}{}. {}", " ".repeat(indent), 
                                   captures.get(2).unwrap().as_str(), item_text)]
                    ));
                    matched = true;
                }
            }

            // If no list item found and we have a current list, finalize it
            if !matched && current_list.is_some() {
                if let Some((list_type, start_pos, _, items)) = current_list.take() {
                    lists.push(TextStructure {
                        id: Uuid::new_v4(),
                        structure_type: list_type,
                        start_offset: start_pos,
                        end_offset: line_start - 1, // End of previous line
                        content: items.join("\n"),
                        nesting_level: 0,
                        parent_id: None,
                        children: Vec::new(),
                        metadata: HashMap::new(),
                    });
                    
                    *id_counter += 1;
                }
            } else if matched {
                // Update end position of current list
                if let Some((_, _, ref mut end_pos, _)) = current_list {
                    *end_pos = line_start + line.len();
                }
            }
        }

        // Finalize any remaining list
        if let Some((list_type, start_pos, end_pos, items)) = current_list {
            lists.push(TextStructure {
                id: Uuid::new_v4(),
                structure_type: list_type,
                start_offset: start_pos,
                end_offset: end_pos,
                content: items.join("\n"),
                nesting_level: 0,
                parent_id: None,
                children: Vec::new(),
                metadata: HashMap::new(),
            });
        }

        Ok(lists)
    }

    /// Detect code block structures
    fn detect_code_blocks(&self, text: &str, id_counter: &mut u32) -> Result<Vec<TextStructure>> {
        let mut code_blocks = Vec::new();
        
        // Fenced code blocks (``` or ~~~)
        let fenced_regex = Regex::new(r"^```(\w+)?\n([\s\S]*?)\n```$").unwrap();
        let tilde_regex = Regex::new(r"^~~~(\w+)?\n([\s\S]*?)\n~~~$").unwrap();
        
        // Indented code blocks (4 spaces or 1 tab)
        let indented_regex = Regex::new(r"^(    |\t)(.*)$").unwrap();

        // Find fenced code blocks
        for captures in fenced_regex.captures_iter(text) {
            let full_match = captures.get(0).unwrap();
            let language = captures.get(1).map(|m| m.as_str().to_string());
            let code_content = captures.get(2).unwrap().as_str();
            
            code_blocks.push(TextStructure {
                id: Uuid::new_v4(),
                structure_type: TextStructureType::CodeBlock { language },
                start_offset: full_match.start(),
                end_offset: full_match.end(),
                content: code_content.to_string(),
                nesting_level: 0,
                parent_id: None,
                children: Vec::new(),
                metadata: HashMap::new(),
            });
            
            *id_counter += 1;
        }

        // Find tilde fenced code blocks
        for captures in tilde_regex.captures_iter(text) {
            let full_match = captures.get(0).unwrap();
            let language = captures.get(1).map(|m| m.as_str().to_string());
            let code_content = captures.get(2).unwrap().as_str();
            
            code_blocks.push(TextStructure {
                id: Uuid::new_v4(),
                structure_type: TextStructureType::CodeBlock { language },
                start_offset: full_match.start(),
                end_offset: full_match.end(),
                content: code_content.to_string(),
                nesting_level: 0,
                parent_id: None,
                children: Vec::new(),
                metadata: HashMap::new(),
            });
            
            *id_counter += 1;
        }

        // Find indented code blocks
        let mut current_code_block: Option<(usize, usize, Vec<String>)> = None;
        
        for (line_num, line) in text.lines().enumerate() {
            let line_start = text.lines().take(line_num).map(|l| l.len() + 1).sum::<usize>();
            
            if indented_regex.is_match(line) {
                let code_line = line.trim_start_matches(|c| c == ' ' || c == '\t');
                
                if let Some((_, _, ref mut lines)) = current_code_block {
                    lines.push(code_line.to_string());
                } else {
                    current_code_block = Some((line_start, line_start + line.len(), vec![code_line.to_string()]));
                }
            } else if line.trim().is_empty() {
                // Empty line - continue code block if we're in one
                if let Some((_, ref mut end_pos, ref mut lines)) = current_code_block {
                    lines.push("".to_string());
                    *end_pos = line_start + line.len();
                }
            } else {
                // Non-code line - finalize current code block
                if let Some((start_pos, end_pos, lines)) = current_code_block.take() {
                    code_blocks.push(TextStructure {
                        id: Uuid::new_v4(),
                        structure_type: TextStructureType::CodeBlock { language: None },
                        start_offset: start_pos,
                        end_offset: end_pos,
                        content: lines.join("\n"),
                        nesting_level: 0,
                        parent_id: None,
                        children: Vec::new(),
                        metadata: HashMap::new(),
                    });
                    
                    *id_counter += 1;
                }
            }
        }

        // Finalize any remaining code block
        if let Some((start_pos, end_pos, lines)) = current_code_block {
            code_blocks.push(TextStructure {
                id: Uuid::new_v4(),
                structure_type: TextStructureType::CodeBlock { language: None },
                start_offset: start_pos,
                end_offset: end_pos,
                content: lines.join("\n"),
                nesting_level: 0,
                parent_id: None,
                children: Vec::new(),
                metadata: HashMap::new(),
            });
        }

        Ok(code_blocks)
    }

    /// Detect table structures
    fn detect_tables(&self, text: &str, id_counter: &mut u32) -> Result<Vec<TextStructure>> {
        let mut tables = Vec::new();
        
        // Simple table detection (pipe-separated)
        let table_row_regex = Regex::new(r"^\|.*\|$").unwrap();
        let table_separator_regex = Regex::new(r"^\|[\s\-:]*\|$").unwrap();

        let mut current_table: Option<(usize, usize, Vec<String>)> = None;

        for (line_num, line) in text.lines().enumerate() {
            let line_start = text.lines().take(line_num).map(|l| l.len() + 1).sum::<usize>();
            
            if table_row_regex.is_match(line) || table_separator_regex.is_match(line) {
                if let Some((_, ref mut end_pos, ref mut rows)) = current_table {
                    rows.push(line.to_string());
                    *end_pos = line_start + line.len();
                } else {
                    current_table = Some((line_start, line_start + line.len(), vec![line.to_string()]));
                }
            } else {
                // Non-table line - finalize current table
                if let Some((start_pos, end_pos, rows)) = current_table.take() {
                    if rows.len() >= 2 { // At least header and separator
                        tables.push(TextStructure {
                            id: Uuid::new_v4(),
                            structure_type: TextStructureType::Table,
                            start_offset: start_pos,
                            end_offset: end_pos,
                            content: rows.join("\n"),
                            nesting_level: 0,
                            parent_id: None,
                            children: Vec::new(),
                            metadata: HashMap::new(),
                        });
                        
                        *id_counter += 1;
                    }
                }
            }
        }

        // Finalize any remaining table
        if let Some((start_pos, end_pos, rows)) = current_table {
            if rows.len() >= 2 {
                tables.push(TextStructure {
                    id: Uuid::new_v4(),
                    structure_type: TextStructureType::Table,
                    start_offset: start_pos,
                    end_offset: end_pos,
                    content: rows.join("\n"),
                    nesting_level: 0,
                    parent_id: None,
                    children: Vec::new(),
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(tables)
    }

    /// Detect quote structures
    fn detect_quotes(&self, text: &str, _language: &str, id_counter: &mut u32) -> Result<Vec<TextStructure>> {
        let mut quotes = Vec::new();
        
        // Block quote pattern (> )
        let blockquote_regex = Regex::new(r"^>\s*(.*)$").unwrap();

        let mut current_quote: Option<(usize, usize, Vec<String>)> = None;

        for (line_num, line) in text.lines().enumerate() {
            let line_start = text.lines().take(line_num).map(|l| l.len() + 1).sum::<usize>();
            
            if let Some(captures) = blockquote_regex.captures(line) {
                let quote_text = captures.get(1).unwrap().as_str();
                
                if let Some((_, ref mut end_pos, ref mut lines)) = current_quote {
                    lines.push(quote_text.to_string());
                    *end_pos = line_start + line.len();
                } else {
                    current_quote = Some((line_start, line_start + line.len(), vec![quote_text.to_string()]));
                }
            } else {
                // Non-quote line - finalize current quote
                if let Some((start_pos, end_pos, lines)) = current_quote.take() {
                    quotes.push(TextStructure {
                        id: Uuid::new_v4(),
                        structure_type: TextStructureType::Quote,
                        start_offset: start_pos,
                        end_offset: end_pos,
                        content: lines.join("\n"),
                        nesting_level: 1,
                        parent_id: None,
                        children: Vec::new(),
                        metadata: HashMap::new(),
                    });
                    
                    *id_counter += 1;
                }
            }
        }

        // Finalize any remaining quote
        if let Some((start_pos, end_pos, lines)) = current_quote {
            quotes.push(TextStructure {
                id: Uuid::new_v4(),
                structure_type: TextStructureType::Quote,
                start_offset: start_pos,
                end_offset: end_pos,
                content: lines.join("\n"),
                nesting_level: 1,
                parent_id: None,
                children: Vec::new(),
                metadata: HashMap::new(),
            });
        }

        Ok(quotes)
    }

    /// Detect paragraph structures
    fn detect_paragraphs(&self, text: &str, existing_structures: &[TextStructure], id_counter: &mut u32) -> Result<Vec<TextStructure>> {
        let mut paragraphs = Vec::new();
        
        // Create a set of covered ranges by existing structures
        let mut covered_ranges: Vec<(usize, usize)> = existing_structures.iter()
            .map(|s| (s.start_offset, s.end_offset))
            .collect();
        covered_ranges.sort_by_key(|&(start, _)| start);

        let mut current_paragraph_start: Option<usize> = None;
        let mut current_paragraph_lines = Vec::new();
        let mut current_offset = 0;

        for line in text.lines() {
            let line_start = current_offset;
            let line_end = current_offset + line.len();
            current_offset = line_end + 1; // +1 for newline

            // Check if this line is covered by any existing structure
            let is_covered = covered_ranges.iter()
                .any(|&(start, end)| line_start >= start && line_end <= end);

            if is_covered {
                // Finalize current paragraph if exists
                if let Some(start_pos) = current_paragraph_start.take() {
                    if !current_paragraph_lines.is_empty() {
                        let content = current_paragraph_lines.join("\n");
                        if content.trim().len() >= self.config.minimum_paragraph_length {
                            paragraphs.push(TextStructure {
                                id: Uuid::new_v4(),
                                structure_type: TextStructureType::Paragraph,
                                start_offset: start_pos,
                                end_offset: line_start - 1,
                                content,
                                nesting_level: 0,
                                parent_id: None,
                                children: Vec::new(),
                                metadata: HashMap::new(),
                            });
                            
                            *id_counter += 1;
                        }
                    }
                    current_paragraph_lines.clear();
                }
            } else if line.trim().is_empty() {
                // Empty line - finalize current paragraph
                if let Some(start_pos) = current_paragraph_start.take() {
                    if !current_paragraph_lines.is_empty() {
                        let content = current_paragraph_lines.join("\n");
                        if content.trim().len() >= self.config.minimum_paragraph_length {
                            paragraphs.push(TextStructure {
                                id: Uuid::new_v4(),
                                structure_type: TextStructureType::Paragraph,
                                start_offset: start_pos,
                                end_offset: line_start - 1,
                                content,
                                nesting_level: 0,
                                parent_id: None,
                                children: Vec::new(),
                                metadata: HashMap::new(),
                            });
                            
                            *id_counter += 1;
                        }
                    }
                    current_paragraph_lines.clear();
                }
            } else {
                // Regular content line
                if current_paragraph_start.is_none() {
                    current_paragraph_start = Some(line_start);
                }
                current_paragraph_lines.push(line.to_string());
            }
        }

        // Finalize any remaining paragraph
        if let Some(start_pos) = current_paragraph_start {
            if !current_paragraph_lines.is_empty() {
                let content = current_paragraph_lines.join("\n");
                if content.trim().len() >= self.config.minimum_paragraph_length {
                    paragraphs.push(TextStructure {
                        id: Uuid::new_v4(),
                        structure_type: TextStructureType::Paragraph,
                        start_offset: start_pos,
                        end_offset: text.len(),
                        content,
                        nesting_level: 0,
                        parent_id: None,
                        children: Vec::new(),
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Ok(paragraphs)
    }

    /// Build hierarchical structure
    fn build_hierarchy(&self, structures: &[TextStructure]) -> StructureHierarchy {
        let mut structure_tree: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let mut root_elements = Vec::new();
        let mut max_depth = 0;

        // Sort structures by nesting level and position
        let mut sorted_structures = structures.to_vec();
        sorted_structures.sort_by(|a, b| {
            a.nesting_level.cmp(&b.nesting_level)
                .then(a.start_offset.cmp(&b.start_offset))
        });

        // Build parent-child relationships
        for structure in &sorted_structures {
            max_depth = max_depth.max(structure.nesting_level);
            
            if structure.nesting_level == 0 {
                root_elements.push(structure.id);
                structure_tree.insert(structure.id, Vec::new());
            } else {
                // Find parent (previous structure with lower nesting level)
                let parent = sorted_structures.iter()
                    .rev()
                    .find(|s| s.nesting_level < structure.nesting_level && 
                            s.start_offset < structure.start_offset);
                
                if let Some(parent_structure) = parent {
                    structure_tree.entry(parent_structure.id)
                        .or_insert_with(Vec::new)
                        .push(structure.id);
                } else {
                    root_elements.push(structure.id);
                }
                
                structure_tree.insert(structure.id, Vec::new());
            }
        }

        StructureHierarchy {
            root_elements,
            max_depth,
            structure_tree,
        }
    }

    /// Calculate statistics about text structure
    fn calculate_statistics(&self, text: &str, structures: &[TextStructure]) -> StructureStatistics {
        let mut element_counts = HashMap::new();
        let mut paragraph_lengths = Vec::new();

        for structure in structures {
            let type_name = match &structure.structure_type {
                TextStructureType::Paragraph => "paragraph",
                TextStructureType::Heading { .. } => "heading",
                TextStructureType::List { .. } => "list",
                TextStructureType::CodeBlock { .. } => "code_block",
                TextStructureType::Table => "table",
                TextStructureType::Quote => "quote",
                TextStructureType::HorizontalRule => "horizontal_rule",
                TextStructureType::LinkReference => "link_reference",
                TextStructureType::ImageReference => "image_reference",
                TextStructureType::Custom(name) => name,
            };
            
            *element_counts.entry(type_name.to_string()).or_insert(0) += 1;
            
            if matches!(structure.structure_type, TextStructureType::Paragraph) {
                paragraph_lengths.push(structure.content.len() as f64);
            }
        }

        let average_paragraph_length = if paragraph_lengths.is_empty() {
            0.0
        } else {
            paragraph_lengths.iter().sum::<f64>() / paragraph_lengths.len() as f64
        };

        // Count sentences, words, and characters
        let sentence_count = text.split('.').count().saturating_sub(1) +
                           text.split('!').count().saturating_sub(1) +
                           text.split('?').count().saturating_sub(1);
        
        let word_count = text.split_whitespace().count();
        let character_count = text.chars().count();

        // Calculate complexity score based on structure diversity
        let complexity_score = (element_counts.len() as f64 / 8.0).min(1.0) * 
                               (structures.len() as f64 / 20.0).min(1.0);

        StructureStatistics {
            total_elements: structures.len(),
            element_counts,
            average_paragraph_length,
            sentence_count,
            word_count,
            character_count,
            complexity_score,
        }
    }

    /// Analyze language-specific features
    async fn analyze_language_features(&self, text: &str, language: &str) -> Result<LanguageSpecificFeatures> {
        let confidence = 0.8; // Simplified confidence for now
        
        // Determine writing direction
        let writing_direction = match language {
            "ar" | "he" | "fa" | "ur" => WritingDirection::RightToLeft,
            "ja" | "zh" | "ko" => WritingDirection::TopToBottom,
            _ => WritingDirection::LeftToRight,
        };

        // Analyze special characters
        let special_characters = self.analyze_special_characters(text);

        // Detect formatting patterns
        let formatting_patterns = if self.config.detect_formatting_patterns {
            self.detect_formatting_patterns(text)?
        } else {
            Vec::new()
        };

        Ok(LanguageSpecificFeatures {
            detected_language: language.to_string(),
            confidence,
            writing_direction,
            special_characters,
            formatting_patterns,
        })
    }

    /// Analyze special characters in text
    fn analyze_special_characters(&self, text: &str) -> Vec<SpecialCharacter> {
        let mut char_map: HashMap<char, (usize, Vec<usize>)> = HashMap::new();

        for (pos, ch) in text.char_indices() {
            if !ch.is_alphanumeric() && !ch.is_whitespace() {
                char_map.entry(ch)
                    .or_insert((0, Vec::new()))
                    .0 += 1;
                char_map.get_mut(&ch).unwrap().1.push(pos);
            }
        }

        char_map.into_iter()
            .map(|(ch, (count, positions))| {
                let context_type = self.classify_character_context(ch);
                SpecialCharacter {
                    character: ch,
                    count,
                    positions,
                    context_type,
                }
            })
            .collect()
    }

    /// Classify character context
    fn classify_character_context(&self, ch: char) -> CharacterContext {
        match ch {
            '.' | ',' | ';' | ':' | '!' | '?' => CharacterContext::Punctuation,
            '"' | '\'' | '`' | '"' | '"' | '\'' | '\'' => CharacterContext::Quote,
            '•' | '◦' | '▪' | '▫' | '‣' => CharacterContext::Bullet,
            '+' | '-' | '=' | '*' | '/' | '^' | '±' | '∞' => CharacterContext::Mathematical,
            '$' | '€' | '£' | '¥' | '¢' => CharacterContext::Currency,
            'á' | 'é' | 'í' | 'ó' | 'ú' | 'ñ' | 'ü' | 'ç' => CharacterContext::Diacritic,
            _ => CharacterContext::Other,
        }
    }

    /// Detect formatting patterns in text
    fn detect_formatting_patterns(&self, text: &str) -> Result<Vec<FormattingPattern>> {
        let mut patterns = Vec::new();

        // Bold patterns
        let bold_patterns = vec![
            (Regex::new(r"\*\*([^*]+)\*\*").unwrap(), "**text**"),
            (Regex::new(r"__([^_]+)__").unwrap(), "__text__"),
        ];

        for (regex, syntax) in bold_patterns {
            let occurrences = regex.captures_iter(text)
                .map(|cap| {
                    let full_match = cap.get(0).unwrap();
                    let inner_text = cap.get(1).unwrap().as_str();
                    PatternOccurrence {
                        start_offset: full_match.start(),
                        end_offset: full_match.end(),
                        text: inner_text.to_string(),
                        syntax_used: syntax.to_string(),
                    }
                })
                .collect::<Vec<_>>();

            if !occurrences.is_empty() {
                patterns.push(FormattingPattern {
                    pattern_type: PatternType::Bold,
                    occurrences,
                    confidence: 0.9,
                });
            }
        }

        // Italic patterns
        let italic_patterns = vec![
            (Regex::new(r"\*([^*]+)\*").unwrap(), "*text*"),
            (Regex::new(r"_([^_]+)_").unwrap(), "_text_"),
        ];

        for (regex, syntax) in italic_patterns {
            let occurrences = regex.captures_iter(text)
                .map(|cap| {
                    let full_match = cap.get(0).unwrap();
                    let inner_text = cap.get(1).unwrap().as_str();
                    PatternOccurrence {
                        start_offset: full_match.start(),
                        end_offset: full_match.end(),
                        text: inner_text.to_string(),
                        syntax_used: syntax.to_string(),
                    }
                })
                .collect::<Vec<_>>();

            if !occurrences.is_empty() {
                patterns.push(FormattingPattern {
                    pattern_type: PatternType::Italic,
                    occurrences,
                    confidence: 0.9,
                });
            }
        }

        // Code patterns
        let code_regex = Regex::new(r"`([^`]+)`").unwrap();
        let code_occurrences = code_regex.captures_iter(text)
            .map(|cap| {
                let full_match = cap.get(0).unwrap();
                let inner_text = cap.get(1).unwrap().as_str();
                PatternOccurrence {
                    start_offset: full_match.start(),
                    end_offset: full_match.end(),
                    text: inner_text.to_string(),
                    syntax_used: "`code`".to_string(),
                }
            })
            .collect::<Vec<_>>();

        if !code_occurrences.is_empty() {
            patterns.push(FormattingPattern {
                pattern_type: PatternType::Code,
                occurrences: code_occurrences,
                confidence: 0.95,
            });
        }

        // Link patterns
        let link_regex = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        let link_occurrences = link_regex.captures_iter(text)
            .map(|cap| {
                let full_match = cap.get(0).unwrap();
                let link_text = cap.get(1).unwrap().as_str();
                PatternOccurrence {
                    start_offset: full_match.start(),
                    end_offset: full_match.end(),
                    text: link_text.to_string(),
                    syntax_used: "[text](url)".to_string(),
                }
            })
            .collect::<Vec<_>>();

        if !link_occurrences.is_empty() {
            patterns.push(FormattingPattern {
                pattern_type: PatternType::Link,
                occurrences: link_occurrences,
                confidence: 0.9,
            });
        }

        Ok(patterns)
    }

    // Language pattern creation methods

    fn create_english_patterns() -> Result<LanguagePatterns> {
        Ok(LanguagePatterns {
            quote_patterns: vec![
                Regex::new(r#""([^"]+)""#).unwrap(),
                Regex::new(r"'([^']+)'").unwrap(),
            ],
            list_patterns: vec![
                Regex::new(r"^\s*[-*+]\s+").unwrap(),
                Regex::new(r"^\s*\d+\.\s+").unwrap(),
            ],
            emphasis_patterns: vec![
                Regex::new(r"\*([^*]+)\*").unwrap(),
                Regex::new(r"_([^_]+)_").unwrap(),
            ],
            special_char_frequencies: [
                ('a', 8.17), ('e', 12.02), ('i', 6.97), ('o', 7.51), ('u', 2.76),
                ('t', 9.10), ('n', 6.75), ('s', 6.33), ('h', 6.09), ('r', 5.99),
            ].iter().cloned().collect(),
        })
    }

    fn create_spanish_patterns() -> Result<LanguagePatterns> {
        Ok(LanguagePatterns {
            quote_patterns: vec![
                Regex::new(r#""([^"]+)""#).unwrap(),
                Regex::new(r"«([^»]+)»").unwrap(),
            ],
            list_patterns: vec![
                Regex::new(r"^\s*[-*•]\s+").unwrap(),
                Regex::new(r"^\s*\d+[.)]\s+").unwrap(),
            ],
            emphasis_patterns: vec![
                Regex::new(r"\*([^*]+)\*").unwrap(),
                Regex::new(r"_([^_]+)_").unwrap(),
            ],
            special_char_frequencies: [
                ('a', 12.53), ('e', 13.68), ('i', 6.25), ('o', 8.68), ('u', 3.93),
                ('ñ', 0.31), ('á', 0.50), ('é', 0.43), ('í', 0.73), ('ó', 0.82), ('ú', 0.16),
            ].iter().cloned().collect(),
        })
    }

    fn create_french_patterns() -> Result<LanguagePatterns> {
        Ok(LanguagePatterns {
            quote_patterns: vec![
                Regex::new(r#"« ([^»]+) »"#).unwrap(),
                Regex::new(r#""([^"]+)""#).unwrap(),
            ],
            list_patterns: vec![
                Regex::new(r"^\s*[-•·]\s+").unwrap(),
                Regex::new(r"^\s*\d+[.)]\s+").unwrap(),
            ],
            emphasis_patterns: vec![
                Regex::new(r"\*([^*]+)\*").unwrap(),
                Regex::new(r"_([^_]+)_").unwrap(),
            ],
            special_char_frequencies: [
                ('a', 7.64), ('e', 14.72), ('i', 8.42), ('o', 5.34), ('u', 6.05),
                ('ç', 0.06), ('à', 0.49), ('é', 1.76), ('è', 0.27), ('ê', 0.22),
                ('î', 0.05), ('ô', 0.04), ('ù', 0.05), ('û', 0.04),
            ].iter().cloned().collect(),
        })
    }

    fn create_german_patterns() -> Result<LanguagePatterns> {
        Ok(LanguagePatterns {
            quote_patterns: vec![
                Regex::new(r#"„([^"]+)""#).unwrap(),
                Regex::new(r#"»([^«]+)«"#).unwrap(),
            ],
            list_patterns: vec![
                Regex::new(r"^\s*[-•·]\s+").unwrap(),
                Regex::new(r"^\s*\d+[.)]\s+").unwrap(),
            ],
            emphasis_patterns: vec![
                Regex::new(r"\*([^*]+)\*").unwrap(),
                Regex::new(r"_([^_]+)_").unwrap(),
            ],
            special_char_frequencies: [
                ('a', 6.51), ('e', 17.40), ('i', 7.55), ('o', 2.51), ('u', 4.35),
                ('ä', 0.54), ('ö', 0.30), ('ü', 0.65), ('ß', 0.31),
            ].iter().cloned().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_structure_analysis() {
        let config = StructureAnalysisConfig::default();
        let analyzer = TextStructureAnalyzer::new(config).unwrap();
        
        let text = "# Heading\n\nThis is a paragraph.\n\n- List item 1\n- List item 2\n\n```rust\nlet x = 5;\n```";
        
        let result = analyzer.analyze_structure(text, Some("en")).await.unwrap();
        
        assert!(result.structures.len() > 0);
        assert!(result.structures.iter().any(|s| matches!(s.structure_type, TextStructureType::Heading { level: 1 })));
        assert!(result.structures.iter().any(|s| matches!(s.structure_type, TextStructureType::Paragraph)));
        assert!(result.structures.iter().any(|s| matches!(s.structure_type, TextStructureType::List { .. })));
        assert!(result.structures.iter().any(|s| matches!(s.structure_type, TextStructureType::CodeBlock { .. })));
    }

    #[tokio::test]
    async fn test_language_detection() {
        let config = StructureAnalysisConfig::default();
        let analyzer = TextStructureAnalyzer::new(config).unwrap();
        
        let spanish_text = "¡Hola! ¿Cómo estás? Ñoño está en España.";
        let detected = analyzer.detect_language(spanish_text).await.unwrap();
        
        // Should detect Spanish due to special characters
        assert_eq!(detected, "es");
    }

    #[test]
    fn test_heading_detection() {
        let config = StructureAnalysisConfig::default();
        let analyzer = TextStructureAnalyzer::new(config).unwrap();
        
        let text = "# Level 1\n## Level 2\n### Level 3";
        let mut counter = 0;
        let headings = analyzer.detect_headings(text, &mut counter).unwrap();
        
        assert_eq!(headings.len(), 3);
        assert!(matches!(headings[0].structure_type, TextStructureType::Heading { level: 1 }));
        assert!(matches!(headings[1].structure_type, TextStructureType::Heading { level: 2 }));
        assert!(matches!(headings[2].structure_type, TextStructureType::Heading { level: 3 }));
    }

    #[test]
    fn test_list_detection() {
        let config = StructureAnalysisConfig::default();
        let analyzer = TextStructureAnalyzer::new(config).unwrap();
        
        let text = "- Item 1\n- Item 2\n\n1. First\n2. Second";
        let mut counter = 0;
        let lists = analyzer.detect_lists(text, &mut counter).unwrap();
        
        assert_eq!(lists.len(), 2);
        assert!(matches!(lists[0].structure_type, TextStructureType::List { list_type: ListType::Unordered }));
        assert!(matches!(lists[1].structure_type, TextStructureType::List { list_type: ListType::Ordered }));
    }

    #[test]
    fn test_special_character_analysis() {
        let config = StructureAnalysisConfig::default();
        let analyzer = TextStructureAnalyzer::new(config).unwrap();
        
        let text = "Hello! How are you? Fine, thanks.";
        let special_chars = analyzer.analyze_special_characters(text);
        
        assert!(special_chars.iter().any(|sc| sc.character == '!' && sc.count == 1));
        assert!(special_chars.iter().any(|sc| sc.character == '?' && sc.count == 1));
        assert!(special_chars.iter().any(|sc| sc.character == ',' && sc.count == 1));
        assert!(special_chars.iter().any(|sc| sc.character == '.' && sc.count == 1));
    }
}