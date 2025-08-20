use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use regex::Regex;
use pulldown_cmark::{
    Parser, Event, Tag, TagEnd, HeadingLevel, CodeBlockKind, Options
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::markdown_text_processor::{
    MarkdownFormat, TextProcessorError
};

/// Markdown AST node representing document structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MarkdownNode {
    Document { children: Vec<MarkdownNode> },
    Heading { level: u8, text: String, id: Option<String>, position: TextRange },
    Paragraph { children: Vec<MarkdownNode>, position: TextRange },
    Text { content: String, position: TextRange },
    Emphasis { children: Vec<MarkdownNode>, position: TextRange },
    Strong { children: Vec<MarkdownNode>, position: TextRange },
    Code { content: String, position: TextRange },
    CodeBlock { language: Option<String>, content: String, position: TextRange },
    Link { url: String, title: Option<String>, children: Vec<MarkdownNode>, position: TextRange },
    Image { url: String, alt: String, title: Option<String>, position: TextRange },
    List { ordered: bool, children: Vec<MarkdownNode>, position: TextRange },
    ListItem { children: Vec<MarkdownNode>, position: TextRange },
    BlockQuote { children: Vec<MarkdownNode>, position: TextRange },
    Table { headers: Vec<String>, rows: Vec<Vec<String>>, position: TextRange },
    HorizontalRule { position: TextRange },
    LineBreak { position: TextRange },
    SoftBreak { position: TextRange },
    Strikethrough { children: Vec<MarkdownNode>, position: TextRange },
    TaskListItem { checked: bool, children: Vec<MarkdownNode>, position: TextRange },
    Footnote { id: String, content: String, position: TextRange },
    FootnoteReference { id: String, position: TextRange },
}

/// Position range in the source text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, position: usize) -> bool {
        position >= self.start && position < self.end
    }

    pub fn overlaps(&self, other: &TextRange) -> bool {
        !(self.end <= other.start || other.end <= self.start)
    }
}

/// Markdown validation error
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationError {
    pub error_type: ValidationErrorType,
    pub message: String,
    pub position: TextRange,
    pub severity: Severity,
    pub suggestions: Vec<String>,
}

/// Types of validation errors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationErrorType {
    SyntaxError,
    MalformedLink,
    BrokenImage,
    InvalidCodeBlock,
    MalformedTable,
    InconsistentHeading,
    UnmatchedDelimiter,
    InvalidHtml,
    AccessibilityIssue,
}

/// Error severity levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Markdown processing configuration
#[derive(Debug, Clone)]
pub struct MarkdownProcessingConfig {
    pub enable_tables: bool,
    pub enable_footnotes: bool,
    pub enable_strikethrough: bool,
    pub enable_tasklists: bool,
    pub enable_smart_punctuation: bool,
    pub enable_heading_attributes: bool,
    pub validate_links: bool,
    pub validate_images: bool,
    pub check_accessibility: bool,
    pub max_heading_level: u8,
    pub allowed_html_tags: HashSet<String>,
    pub custom_extensions: Vec<String>,
}

impl Default for MarkdownProcessingConfig {
    fn default() -> Self {
        let mut allowed_tags = HashSet::new();
        allowed_tags.extend([
            "strong", "em", "u", "s", "code", "kbd", "samp", "var",
            "mark", "ins", "del", "sup", "sub", "small", "big",
            "br", "hr", "span", "div", "p", "blockquote"
        ].iter().map(|s| s.to_string()));

        Self {
            enable_tables: true,
            enable_footnotes: true,
            enable_strikethrough: true,
            enable_tasklists: true,
            enable_smart_punctuation: true,
            enable_heading_attributes: true,
            validate_links: true,
            validate_images: true,
            check_accessibility: true,
            max_heading_level: 6,
            allowed_html_tags: allowed_tags,
            custom_extensions: Vec::new(),
        }
    }
}

/// Format detection result
#[derive(Debug, Clone, PartialEq)]
pub struct FormatDetection {
    pub format_type: MarkdownFormat,
    pub range: TextRange,
    pub nested_formats: Vec<FormatDetection>,
    pub is_well_formed: bool,
}

/// Processing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStatistics {
    pub word_count: usize,
    pub character_count: usize,
    pub line_count: usize,
    pub heading_count: HashMap<u8, usize>,
    pub link_count: usize,
    pub image_count: usize,
    pub code_block_count: usize,
    pub table_count: usize,
    pub footnote_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
}

/// Main markdown processing engine
pub struct MarkdownProcessor {
    config: MarkdownProcessingConfig,
    cache: Arc<RwLock<HashMap<String, MarkdownNode>>>,
    validator_cache: Arc<RwLock<HashMap<String, Vec<ValidationError>>>>,
    link_validator: Option<Box<dyn LinkValidator + Send + Sync>>,
    custom_parsers: HashMap<String, Box<dyn CustomParser + Send + Sync>>,
}

impl MarkdownProcessor {
    /// Create a new markdown processor
    pub fn new() -> Self {
        Self {
            config: MarkdownProcessingConfig::default(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            validator_cache: Arc::new(RwLock::new(HashMap::new())),
            link_validator: None,
            custom_parsers: HashMap::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: MarkdownProcessingConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            validator_cache: Arc::new(RwLock::new(HashMap::new())),
            link_validator: None,
            custom_parsers: HashMap::new(),
        }
    }

    /// Set link validator
    pub fn set_link_validator<V: LinkValidator + Send + Sync + 'static>(&mut self, validator: V) {
        self.link_validator = Some(Box::new(validator));
    }

    /// Add custom parser
    pub fn add_custom_parser<P: CustomParser + Send + Sync + 'static>(&mut self, name: String, parser: P) {
        self.custom_parsers.insert(name, Box::new(parser));
    }

    /// Parse markdown text into AST
    pub async fn parse(&self, markdown: &str) -> Result<MarkdownNode, MarkdownProcessorError> {
        // Check cache first
        let cache_key = format!("{:x}", md5::compute(markdown));
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let mut options = Options::empty();
        
        if self.config.enable_tables {
            options.insert(Options::ENABLE_TABLES);
        }
        if self.config.enable_footnotes {
            options.insert(Options::ENABLE_FOOTNOTES);
        }
        if self.config.enable_strikethrough {
            options.insert(Options::ENABLE_STRIKETHROUGH);
        }
        if self.config.enable_tasklists {
            options.insert(Options::ENABLE_TASKLISTS);
        }
        if self.config.enable_smart_punctuation {
            options.insert(Options::ENABLE_SMART_PUNCTUATION);
        }
        if self.config.enable_heading_attributes {
            options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
        }

        let parser = Parser::new_ext(markdown, options);
        let ast = self.build_ast(parser, markdown)?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, ast.clone());
        }

        Ok(ast)
    }

    /// Build AST from parser events
    fn build_ast(&self, parser: Parser, _source: &str) -> Result<MarkdownNode, MarkdownProcessorError> {
        let mut stack: Vec<MarkdownNode> = Vec::new();
        let mut root_children = Vec::new();
        let mut _current_position = 0;

        for (event, range) in parser.into_offset_iter() {
            let text_range = TextRange::new(range.start, range.end);
            _current_position = range.end;

            match event {
                Event::Start(tag) => {
                    let node = self.create_node_from_tag(tag, text_range)?;
                    stack.push(node);
                }
                Event::End(tag_end) => {
                    if let Some(mut node) = stack.pop() {
                        // Collect children from the stack if this is a container
                        let children = self.collect_children(&mut stack, &tag_end);
                        self.set_node_children(&mut node, children);
                        
                        if stack.is_empty() {
                            root_children.push(node);
                        } else {
                            // This is a child of the current top-level node
                            if let Some(parent) = stack.last_mut() {
                                self.add_child_to_node(parent, node);
                            }
                        }
                    }
                }
                Event::Text(text) => {
                    let node = MarkdownNode::Text {
                        content: text.to_string(),
                        position: text_range,
                    };
                    
                    if let Some(parent) = stack.last_mut() {
                        self.add_child_to_node(parent, node);
                    } else {
                        root_children.push(node);
                    }
                }
                Event::Code(code) => {
                    let node = MarkdownNode::Code {
                        content: code.to_string(),
                        position: text_range,
                    };
                    
                    if let Some(parent) = stack.last_mut() {
                        self.add_child_to_node(parent, node);
                    } else {
                        root_children.push(node);
                    }
                }
                Event::Html(html) => {
                    // Handle HTML content - could be parsed further
                    let node = MarkdownNode::Text {
                        content: html.to_string(),
                        position: text_range,
                    };
                    
                    if let Some(parent) = stack.last_mut() {
                        self.add_child_to_node(parent, node);
                    } else {
                        root_children.push(node);
                    }
                }
                Event::FootnoteReference(name) => {
                    let node = MarkdownNode::FootnoteReference {
                        id: name.to_string(),
                        position: text_range,
                    };
                    
                    if let Some(parent) = stack.last_mut() {
                        self.add_child_to_node(parent, node);
                    } else {
                        root_children.push(node);
                    }
                }
                Event::SoftBreak => {
                    let node = MarkdownNode::SoftBreak { position: text_range };
                    
                    if let Some(parent) = stack.last_mut() {
                        self.add_child_to_node(parent, node);
                    } else {
                        root_children.push(node);
                    }
                }
                Event::HardBreak => {
                    let node = MarkdownNode::LineBreak { position: text_range };
                    
                    if let Some(parent) = stack.last_mut() {
                        self.add_child_to_node(parent, node);
                    } else {
                        root_children.push(node);
                    }
                }
                Event::Rule => {
                    let node = MarkdownNode::HorizontalRule { position: text_range };
                    
                    if let Some(parent) = stack.last_mut() {
                        self.add_child_to_node(parent, node);
                    } else {
                        root_children.push(node);
                    }
                }
                Event::TaskListMarker(checked) => {
                    // Handle task list markers - usually part of list items
                    if let Some(parent) = stack.last_mut() {
                        if let MarkdownNode::ListItem { .. } = parent {
                            // Convert to task list item
                            *parent = MarkdownNode::TaskListItem {
                                checked,
                                children: Vec::new(),
                                position: text_range,
                            };
                        }
                    }
                }
                Event::InlineHtml(html) => {
                    // Handle inline HTML content
                    let node = MarkdownNode::Text {
                        content: html.to_string(),
                        position: text_range,
                    };
                    
                    if stack.is_empty() {
                        root_children.push(node);
                    } else {
                        if let Some(parent) = stack.last_mut() {
                            self.add_child_to_node(parent, node);
                        }
                    }
                }
            }
        }

        Ok(MarkdownNode::Document {
            children: root_children,
        })
    }

    /// Create node from opening tag
    fn create_node_from_tag(&self, tag: Tag, position: TextRange) -> Result<MarkdownNode, MarkdownProcessorError> {
        let node = match tag {
            Tag::Paragraph => MarkdownNode::Paragraph {
                children: Vec::new(),
                position,
            },
            Tag::Heading { level, id, classes: _, attrs: _ } => {
                let heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                MarkdownNode::Heading {
                    level: heading_level,
                    text: String::new(),
                    id: id.map(|s| s.to_string()),
                    position,
                }
            },
            Tag::BlockQuote => MarkdownNode::BlockQuote {
                children: Vec::new(),
                position,
            },
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Indented => None,
                    CodeBlockKind::Fenced(lang) => {
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang.to_string())
                        }
                    }
                };
                MarkdownNode::CodeBlock {
                    language,
                    content: String::new(),
                    position,
                }
            },
            Tag::List(start_number) => MarkdownNode::List {
                ordered: start_number.is_some(),
                children: Vec::new(),
                position,
            },
            Tag::Item => MarkdownNode::ListItem {
                children: Vec::new(),
                position,
            },
            Tag::Emphasis => MarkdownNode::Emphasis {
                children: Vec::new(),
                position,
            },
            Tag::Strong => MarkdownNode::Strong {
                children: Vec::new(),
                position,
            },
            Tag::Strikethrough => MarkdownNode::Strikethrough {
                children: Vec::new(),
                position,
            },
            Tag::Link { link_type: _, dest_url, title, id: _ } => MarkdownNode::Link {
                url: dest_url.to_string(),
                title: if title.is_empty() { None } else { Some(title.to_string()) },
                children: Vec::new(),
                position,
            },
            Tag::Image { link_type: _, dest_url, title, id: _ } => MarkdownNode::Image {
                url: dest_url.to_string(),
                alt: String::new(), // Will be filled from content
                title: if title.is_empty() { None } else { Some(title.to_string()) },
                position,
            },
            Tag::Table(_) => MarkdownNode::Table {
                headers: Vec::new(),
                rows: Vec::new(),
                position,
            },
            Tag::TableHead => MarkdownNode::Text {
                content: String::new(),
                position,
            },
            Tag::TableRow => MarkdownNode::Text {
                content: String::new(),
                position,
            },
            Tag::TableCell => MarkdownNode::Text {
                content: String::new(),
                position,
            },
            Tag::FootnoteDefinition(name) => MarkdownNode::Footnote {
                id: name.to_string(),
                content: String::new(),
                position,
            },
            Tag::HtmlBlock => MarkdownNode::Text {
                content: String::new(),
                position,
            },
            Tag::MetadataBlock(_) => MarkdownNode::Text {
                content: String::new(),
                position,
            },
        };

        Ok(node)
    }

    /// Collect children from stack based on tag end
    fn collect_children(&self, _stack: &mut Vec<MarkdownNode>, _tag_end: &TagEnd) -> Vec<MarkdownNode> {
        // This would collect children based on the specific tag type
        Vec::new()
    }

    /// Set children for a node
    fn set_node_children(&self, node: &mut MarkdownNode, children: Vec<MarkdownNode>) {
        match node {
            MarkdownNode::Document { children: ref mut c } |
            MarkdownNode::Paragraph { children: ref mut c, .. } |
            MarkdownNode::Emphasis { children: ref mut c, .. } |
            MarkdownNode::Strong { children: ref mut c, .. } |
            MarkdownNode::Link { children: ref mut c, .. } |
            MarkdownNode::List { children: ref mut c, .. } |
            MarkdownNode::ListItem { children: ref mut c, .. } |
            MarkdownNode::BlockQuote { children: ref mut c, .. } |
            MarkdownNode::Strikethrough { children: ref mut c, .. } |
            MarkdownNode::TaskListItem { children: ref mut c, .. } => {
                *c = children;
            }
            _ => {} // Other nodes don't have children
        }
    }

    /// Add child to a node
    fn add_child_to_node(&self, parent: &mut MarkdownNode, child: MarkdownNode) {
        match parent {
            MarkdownNode::Document { children } |
            MarkdownNode::Paragraph { children, .. } |
            MarkdownNode::Emphasis { children, .. } |
            MarkdownNode::Strong { children, .. } |
            MarkdownNode::Link { children, .. } |
            MarkdownNode::List { children, .. } |
            MarkdownNode::ListItem { children, .. } |
            MarkdownNode::BlockQuote { children, .. } |
            MarkdownNode::Strikethrough { children, .. } |
            MarkdownNode::TaskListItem { children, .. } => {
                children.push(child);
            }
            _ => {} // Other nodes don't accept children
        }
    }

    /// Validate markdown content
    pub async fn validate(&self, markdown: &str) -> Result<Vec<ValidationError>, MarkdownProcessorError> {
        // Check cache first
        let cache_key = format!("{:x}", md5::compute(markdown));
        {
            let cache = self.validator_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let mut errors = Vec::new();

        // Parse and validate simultaneously
        let ast = self.parse(markdown).await?;
        
        // Structural validation
        self.validate_structure(&ast, &mut errors);
        
        // Link validation
        if self.config.validate_links {
            self.validate_links(&ast, &mut errors).await;
        }

        // Image validation
        if self.config.validate_images {
            self.validate_images(&ast, &mut errors).await;
        }

        // Accessibility validation
        if self.config.check_accessibility {
            self.validate_accessibility(&ast, &mut errors);
        }

        // Cache the result
        {
            let mut cache = self.validator_cache.write().await;
            cache.insert(cache_key, errors.clone());
        }

        Ok(errors)
    }

    /// Validate document structure
    fn validate_structure(&self, node: &MarkdownNode, errors: &mut Vec<ValidationError>) {
        match node {
            MarkdownNode::Document { children } => {
                // Check for proper heading hierarchy
                self.validate_heading_hierarchy(children, errors);
                
                // Recursively validate children
                for child in children {
                    self.validate_structure(child, errors);
                }
            }
            MarkdownNode::Heading { level, position, .. } => {
                if *level > self.config.max_heading_level {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::InconsistentHeading,
                        message: format!("Heading level {} exceeds maximum allowed level {}", level, self.config.max_heading_level),
                        position: *position,
                        severity: Severity::Warning,
                        suggestions: vec![format!("Use heading level {} or lower", self.config.max_heading_level)],
                    });
                }
            }
            MarkdownNode::CodeBlock { language, content, position } => {
                if let Some(lang) = language {
                    if !self.is_valid_language_identifier(lang) {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::InvalidCodeBlock,
                            message: format!("Unknown or invalid language identifier: '{}'", lang),
                            position: *position,
                            severity: Severity::Warning,
                            suggestions: vec!["Use a standard language identifier or leave empty".to_string()],
                        });
                    }
                }
                
                // Validate code content for common issues
                if content.trim().is_empty() {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::InvalidCodeBlock,
                        message: "Empty code block".to_string(),
                        position: *position,
                        severity: Severity::Info,
                        suggestions: vec!["Add code content or remove the code block".to_string()],
                    });
                }
            }
            MarkdownNode::Link { url, position, .. } => {
                if url.is_empty() {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::MalformedLink,
                        message: "Link has empty URL".to_string(),
                        position: *position,
                        severity: Severity::Error,
                        suggestions: vec!["Provide a valid URL for the link".to_string()],
                    });
                }
            }
            MarkdownNode::Image { url, alt, position, .. } => {
                if url.is_empty() {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::BrokenImage,
                        message: "Image has empty URL".to_string(),
                        position: *position,
                        severity: Severity::Error,
                        suggestions: vec!["Provide a valid URL for the image".to_string()],
                    });
                }
                
                if alt.is_empty() {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::AccessibilityIssue,
                        message: "Image missing alt text".to_string(),
                        position: *position,
                        severity: Severity::Warning,
                        suggestions: vec!["Add descriptive alt text for accessibility".to_string()],
                    });
                }
            }
            // Recursively validate other nodes with children
            _ => {
                if let Some(children) = self.get_node_children(node) {
                    for child in children {
                        self.validate_structure(child, errors);
                    }
                }
            }
        }
    }

    /// Validate heading hierarchy
    fn validate_heading_hierarchy(&self, nodes: &[MarkdownNode], errors: &mut Vec<ValidationError>) {
        let mut previous_level = 0u8;
        
        for node in nodes {
            if let MarkdownNode::Heading { level, position, .. } = node {
                if previous_level > 0 && *level > previous_level + 1 {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::InconsistentHeading,
                        message: format!("Heading level {} skips from level {}", level, previous_level),
                        position: *position,
                        severity: Severity::Warning,
                        suggestions: vec![format!("Use heading level {} to maintain hierarchy", previous_level + 1)],
                    });
                }
                previous_level = *level;
            }
        }
    }

    /// Validate links
    async fn validate_links(&self, node: &MarkdownNode, errors: &mut Vec<ValidationError>) {
        match node {
            MarkdownNode::Link { url, position, .. } => {
                if let Some(validator) = &self.link_validator {
                    if !validator.validate_link(url).await {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::MalformedLink,
                            message: format!("Link validation failed for: {}", url),
                            position: *position,
                            severity: Severity::Warning,
                            suggestions: vec!["Check if the link is accessible and correct".to_string()],
                        });
                    }
                }
            }
            _ => {
                if let Some(children) = self.get_node_children(node) {
                    for child in children {
                        Box::pin(self.validate_links(child, errors)).await;
                    }
                }
            }
        }
    }

    /// Validate images
    async fn validate_images(&self, node: &MarkdownNode, errors: &mut Vec<ValidationError>) {
        match node {
            MarkdownNode::Image { url, position, .. } => {
                if let Some(validator) = &self.link_validator {
                    if !validator.validate_link(url).await {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::BrokenImage,
                            message: format!("Image validation failed for: {}", url),
                            position: *position,
                            severity: Severity::Warning,
                            suggestions: vec!["Check if the image URL is accessible and correct".to_string()],
                        });
                    }
                }
            }
            _ => {
                if let Some(children) = self.get_node_children(node) {
                    for child in children {
                        Box::pin(self.validate_images(child, errors)).await;
                    }
                }
            }
        }
    }

    /// Validate accessibility
    fn validate_accessibility(&self, node: &MarkdownNode, errors: &mut Vec<ValidationError>) {
        match node {
            MarkdownNode::Image { alt, position, .. } => {
                if alt.is_empty() {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::AccessibilityIssue,
                        message: "Image missing alt text for accessibility".to_string(),
                        position: *position,
                        severity: Severity::Warning,
                        suggestions: vec!["Add descriptive alt text".to_string()],
                    });
                } else if alt.len() > 200 {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::AccessibilityIssue,
                        message: "Alt text is too long (over 200 characters)".to_string(),
                        position: *position,
                        severity: Severity::Info,
                        suggestions: vec!["Consider shortening alt text or using a caption".to_string()],
                    });
                }
            }
            MarkdownNode::Heading { text, level, position, .. } => {
                if text.is_empty() {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::AccessibilityIssue,
                        message: "Empty heading text".to_string(),
                        position: *position,
                        severity: Severity::Error,
                        suggestions: vec!["Add descriptive heading text".to_string()],
                    });
                }
                
                if text.len() > 100 {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::AccessibilityIssue,
                        message: format!("Heading level {} text is very long", level),
                        position: *position,
                        severity: Severity::Info,
                        suggestions: vec!["Consider shortening heading text".to_string()],
                    });
                }
            }
            MarkdownNode::Link { children, position, .. } => {
                let link_text = self.extract_text_content(children);
                if link_text.is_empty() {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::AccessibilityIssue,
                        message: "Link has no descriptive text".to_string(),
                        position: *position,
                        severity: Severity::Warning,
                        suggestions: vec!["Add descriptive link text".to_string()],
                    });
                } else if link_text.to_lowercase() == "click here" || link_text.to_lowercase() == "read more" {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::AccessibilityIssue,
                        message: "Link text is not descriptive".to_string(),
                        position: *position,
                        severity: Severity::Info,
                        suggestions: vec!["Use more descriptive link text".to_string()],
                    });
                }
            }
            _ => {
                if let Some(children) = self.get_node_children(node) {
                    for child in children {
                        self.validate_accessibility(child, errors);
                    }
                }
            }
        }
    }

    /// Extract text content from nodes
    fn extract_text_content(&self, nodes: &[MarkdownNode]) -> String {
        let mut text = String::new();
        for node in nodes {
            match node {
                MarkdownNode::Text { content, .. } => text.push_str(content),
                _ => {
                    if let Some(children) = self.get_node_children(node) {
                        text.push_str(&self.extract_text_content(children));
                    }
                }
            }
        }
        text
    }

    /// Get children of a node
    fn get_node_children<'a>(&self, node: &'a MarkdownNode) -> Option<&'a [MarkdownNode]> {
        match node {
            MarkdownNode::Document { children } |
            MarkdownNode::Paragraph { children, .. } |
            MarkdownNode::Emphasis { children, .. } |
            MarkdownNode::Strong { children, .. } |
            MarkdownNode::Link { children, .. } |
            MarkdownNode::List { children, .. } |
            MarkdownNode::ListItem { children, .. } |
            MarkdownNode::BlockQuote { children, .. } |
            MarkdownNode::Strikethrough { children, .. } |
            MarkdownNode::TaskListItem { children, .. } => Some(children),
            _ => None,
        }
    }

    /// Check if language identifier is valid
    fn is_valid_language_identifier(&self, lang: &str) -> bool {
        // Common programming language identifiers
        const VALID_LANGUAGES: &[&str] = &[
            "rust", "rs", "python", "py", "javascript", "js", "typescript", "ts",
            "java", "c", "cpp", "c++", "csharp", "cs", "go", "php", "ruby", "rb",
            "swift", "kotlin", "scala", "clojure", "haskell", "erlang", "elixir",
            "r", "matlab", "sql", "html", "css", "scss", "sass", "less",
            "xml", "json", "yaml", "yml", "toml", "ini", "bash", "sh",
            "powershell", "ps1", "dockerfile", "makefile", "cmake", "tex",
            "markdown", "md", "text", "txt", "plain"
        ];
        
        VALID_LANGUAGES.contains(&lang.to_lowercase().as_str())
    }

    /// Detect existing formatting in text range
    pub fn detect_formatting(&self, markdown: &str, range: TextRange) -> Vec<FormatDetection> {
        let text_slice = &markdown[range.start..range.end.min(markdown.len())];
        let mut detections = Vec::new();

        // Detect bold formatting
        self.detect_pattern_formatting(
            text_slice,
            r"\*\*(.*?)\*\*",
            MarkdownFormat::Bold,
            range.start,
            &mut detections,
        );

        // Detect italic formatting
        self.detect_pattern_formatting(
            text_slice,
            r"\*(.*?)\*",
            MarkdownFormat::Italic,
            range.start,
            &mut detections,
        );

        // Detect code formatting
        self.detect_pattern_formatting(
            text_slice,
            r"`(.*?)`",
            MarkdownFormat::Code,
            range.start,
            &mut detections,
        );

        // Detect strikethrough formatting
        self.detect_pattern_formatting(
            text_slice,
            r"~~(.*?)~~",
            MarkdownFormat::Strikethrough,
            range.start,
            &mut detections,
        );

        // Detect links
        self.detect_link_formatting(text_slice, range.start, &mut detections);

        // Sort by position
        detections.sort_by_key(|d| d.range.start);

        detections
    }

    /// Detect pattern-based formatting
    fn detect_pattern_formatting(
        &self,
        text: &str,
        pattern: &str,
        format_type: MarkdownFormat,
        offset: usize,
        detections: &mut Vec<FormatDetection>,
    ) {
        if let Ok(regex) = Regex::new(pattern) {
            for mat in regex.find_iter(text) {
                let detection = FormatDetection {
                    format_type: format_type.clone(),
                    range: TextRange::new(offset + mat.start(), offset + mat.end()),
                    nested_formats: Vec::new(),
                    is_well_formed: true,
                };
                detections.push(detection);
            }
        }
    }

    /// Detect link formatting
    fn detect_link_formatting(
        &self,
        text: &str,
        offset: usize,
        detections: &mut Vec<FormatDetection>,
    ) {
        // Match markdown links: [text](url)
        if let Ok(regex) = Regex::new(r"\[([^\]]*)\]\(([^)]*)\)") {
            for mat in regex.find_iter(text) {
                if let Some(caps) = regex.captures(mat.as_str()) {
                    let url = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
                    let detection = FormatDetection {
                        format_type: MarkdownFormat::Link {
                            url,
                            title: None,
                        },
                        range: TextRange::new(offset + mat.start(), offset + mat.end()),
                        nested_formats: Vec::new(),
                        is_well_formed: true,
                    };
                    detections.push(detection);
                }
            }
        }
    }

    /// Apply formatting while preserving document structure
    pub async fn apply_formatting_to_range(
        &self,
        markdown: &str,
        range: TextRange,
        format: MarkdownFormat,
    ) -> Result<String, MarkdownProcessorError> {
        let mut result = markdown.to_string();
        
        // Check for existing formatting conflicts
        let existing_formats = self.detect_formatting(markdown, range);
        
        // Handle conflicts intelligently
        for existing in &existing_formats {
            if existing.range.overlaps(&range) {
                // Remove conflicting formatting first
                if self.formats_conflict(&existing.format_type, &format) {
                    result = self.remove_formatting_from_range(&result, existing.range)?;
                }
            }
        }

        // Apply new formatting
        let text_to_format = &result[range.start..range.end.min(result.len())];
        let formatted_text = self.format_text(text_to_format, &format);
        
        result.replace_range(range.start..range.end.min(result.len()), &formatted_text);
        
        Ok(result)
    }

    /// Check if two formats conflict
    fn formats_conflict(&self, format1: &MarkdownFormat, format2: &MarkdownFormat) -> bool {
        use MarkdownFormat::*;
        match (format1, format2) {
            (Bold, Bold) | (Italic, Italic) | (Code, Code) | (Strikethrough, Strikethrough) => true,
            (Code, _) | (_, Code) => true, // Code formatting conflicts with most others
            _ => false,
        }
    }

    /// Remove formatting from range
    fn remove_formatting_from_range(&self, markdown: &str, range: TextRange) -> Result<String, MarkdownProcessorError> {
        let mut result = markdown.to_string();
        let text_slice = &markdown[range.start..range.end.min(markdown.len())];
        
        // Remove common markdown formatting patterns
        let cleaned = text_slice
            .replace("**", "")  // Bold
            .replace("*", "")   // Italic
            .replace("`", "")   // Code
            .replace("~~", ""); // Strikethrough
            
        result.replace_range(range.start..range.end.min(markdown.len()), &cleaned);
        Ok(result)
    }

    /// Format text with given format
    fn format_text(&self, text: &str, format: &MarkdownFormat) -> String {
        match format {
            MarkdownFormat::Bold => format!("**{}**", text),
            MarkdownFormat::Italic => format!("*{}*", text),
            MarkdownFormat::Code => format!("`{}`", text),
            MarkdownFormat::CodeBlock { language } => {
                let lang = language.as_deref().unwrap_or("");
                format!("```{}\n{}\n```", lang, text)
            }
            MarkdownFormat::Heading { level } => {
                let hash_count = "#".repeat(*level as usize);
                format!("{} {}", hash_count, text)
            }
            MarkdownFormat::Link { url, title } => {
                if let Some(title_text) = title {
                    format!("[{}]({} \"{}\")", text, url, title_text)
                } else {
                    format!("[{}]({})", text, url)
                }
            }
            MarkdownFormat::Image { url, alt, title } => {
                if let Some(title_text) = title {
                    format!("![{}]({} \"{}\")", alt, url, title_text)
                } else {
                    format!("![{}]({})", alt, url)
                }
            }
            MarkdownFormat::Strikethrough => format!("~~{}~~", text),
            MarkdownFormat::Underline => format!("<u>{}</u>", text),
            MarkdownFormat::BlockQuote => {
                text.lines()
                    .map(|line| format!("> {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            MarkdownFormat::UnorderedList => {
                text.lines()
                    .map(|line| format!("- {}", line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            MarkdownFormat::OrderedList => {
                text.lines()
                    .enumerate()
                    .map(|(i, line)| format!("{}. {}", i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            MarkdownFormat::HorizontalRule => "---".to_string(),
            MarkdownFormat::Table => format!("| {} |\n|---|\n", text),
        }
    }

    /// Generate processing statistics
    pub async fn generate_statistics(&self, markdown: &str) -> Result<ProcessingStatistics, MarkdownProcessorError> {
        let ast = self.parse(markdown).await?;
        let mut stats = ProcessingStatistics {
            word_count: 0,
            character_count: markdown.len(),
            line_count: markdown.lines().count(),
            heading_count: HashMap::new(),
            link_count: 0,
            image_count: 0,
            code_block_count: 0,
            table_count: 0,
            footnote_count: 0,
            error_count: 0,
            warning_count: 0,
        };

        // Count words
        stats.word_count = markdown.split_whitespace().count();

        // Analyze AST
        self.analyze_node_for_stats(&ast, &mut stats);

        // Get validation errors for error/warning counts
        let validation_errors = self.validate(markdown).await?;
        for error in validation_errors {
            match error.severity {
                Severity::Error => stats.error_count += 1,
                Severity::Warning => stats.warning_count += 1,
                _ => {}
            }
        }

        Ok(stats)
    }

    /// Analyze node for statistics
    fn analyze_node_for_stats(&self, node: &MarkdownNode, stats: &mut ProcessingStatistics) {
        match node {
            MarkdownNode::Heading { level, .. } => {
                *stats.heading_count.entry(*level).or_insert(0) += 1;
            }
            MarkdownNode::Link { .. } => {
                stats.link_count += 1;
            }
            MarkdownNode::Image { .. } => {
                stats.image_count += 1;
            }
            MarkdownNode::CodeBlock { .. } => {
                stats.code_block_count += 1;
            }
            MarkdownNode::Table { .. } => {
                stats.table_count += 1;
            }
            MarkdownNode::Footnote { .. } => {
                stats.footnote_count += 1;
            }
            _ => {}
        }

        // Recursively analyze children
        if let Some(children) = self.get_node_children(node) {
            for child in children {
                self.analyze_node_for_stats(child, stats);
            }
        }
    }

    /// Convert AST back to markdown
    pub fn ast_to_markdown(&self, ast: &MarkdownNode) -> Result<String, MarkdownProcessorError> {
        let mut output = String::new();
        self.render_node_to_markdown(ast, &mut output, 0)?;
        Ok(output)
    }

    /// Render node to markdown
    fn render_node_to_markdown(&self, node: &MarkdownNode, output: &mut String, depth: usize) -> Result<(), MarkdownProcessorError> {
        match node {
            MarkdownNode::Document { children } => {
                for child in children {
                    self.render_node_to_markdown(child, output, depth)?;
                }
            }
            MarkdownNode::Heading { level, text, .. } => {
                output.push_str(&"#".repeat(*level as usize));
                output.push(' ');
                output.push_str(text);
                output.push('\n');
            }
            MarkdownNode::Paragraph { children, .. } => {
                for child in children {
                    self.render_node_to_markdown(child, output, depth)?;
                }
                output.push('\n');
            }
            MarkdownNode::Text { content, .. } => {
                output.push_str(content);
            }
            MarkdownNode::Emphasis { children, .. } => {
                output.push('*');
                for child in children {
                    self.render_node_to_markdown(child, output, depth)?;
                }
                output.push('*');
            }
            MarkdownNode::Strong { children, .. } => {
                output.push_str("**");
                for child in children {
                    self.render_node_to_markdown(child, output, depth)?;
                }
                output.push_str("**");
            }
            MarkdownNode::Code { content, .. } => {
                output.push('`');
                output.push_str(content);
                output.push('`');
            }
            MarkdownNode::CodeBlock { language, content, .. } => {
                output.push_str("```");
                if let Some(lang) = language {
                    output.push_str(lang);
                }
                output.push('\n');
                output.push_str(content);
                output.push_str("\n```\n");
            }
            MarkdownNode::Link { url, title, children, .. } => {
                output.push('[');
                for child in children {
                    self.render_node_to_markdown(child, output, depth)?;
                }
                output.push_str("](");
                output.push_str(url);
                if let Some(title_text) = title {
                    output.push_str(" \"");
                    output.push_str(title_text);
                    output.push('"');
                }
                output.push(')');
            }
            MarkdownNode::Image { url, alt, title, .. } => {
                output.push_str("![");
                output.push_str(alt);
                output.push_str("](");
                output.push_str(url);
                if let Some(title_text) = title {
                    output.push_str(" \"");
                    output.push_str(title_text);
                    output.push('"');
                }
                output.push(')');
            }
            MarkdownNode::List { ordered, children, .. } => {
                for (i, child) in children.iter().enumerate() {
                    if *ordered {
                        output.push_str(&format!("{}. ", i + 1));
                    } else {
                        output.push_str("- ");
                    }
                    self.render_node_to_markdown(child, output, depth + 1)?;
                }
            }
            MarkdownNode::ListItem { children, .. } => {
                for child in children {
                    self.render_node_to_markdown(child, output, depth)?;
                }
                output.push('\n');
            }
            MarkdownNode::BlockQuote { children, .. } => {
                for child in children {
                    output.push_str("> ");
                    self.render_node_to_markdown(child, output, depth)?;
                }
            }
            MarkdownNode::HorizontalRule { .. } => {
                output.push_str("---\n");
            }
            MarkdownNode::LineBreak { .. } => {
                output.push_str("  \n");
            }
            MarkdownNode::SoftBreak { .. } => {
                output.push('\n');
            }
            MarkdownNode::Strikethrough { children, .. } => {
                output.push_str("~~");
                for child in children {
                    self.render_node_to_markdown(child, output, depth)?;
                }
                output.push_str("~~");
            }
            MarkdownNode::TaskListItem { checked, children, .. } => {
                if *checked {
                    output.push_str("- [x] ");
                } else {
                    output.push_str("- [ ] ");
                }
                for child in children {
                    self.render_node_to_markdown(child, output, depth)?;
                }
                output.push('\n');
            }
            MarkdownNode::Footnote { id, content, .. } => {
                output.push_str(&format!("[^{}]: {}\n", id, content));
            }
            MarkdownNode::FootnoteReference { id, .. } => {
                output.push_str(&format!("[^{}]", id));
            }
            MarkdownNode::Table { headers, rows, .. } => {
                // Render table headers
                output.push('|');
                for header in headers {
                    output.push(' ');
                    output.push_str(header);
                    output.push_str(" |");
                }
                output.push('\n');
                
                // Render separator
                output.push('|');
                for _ in headers {
                    output.push_str("---|");
                }
                output.push('\n');
                
                // Render rows
                for row in rows {
                    output.push('|');
                    for cell in row {
                        output.push(' ');
                        output.push_str(cell);
                        output.push_str(" |");
                    }
                    output.push('\n');
                }
            }
        }
        Ok(())
    }

    /// Clear caches
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        
        let mut validator_cache = self.validator_cache.write().await;
        validator_cache.clear();
    }
}

/// Trait for link validation
#[async_trait::async_trait]
pub trait LinkValidator {
    async fn validate_link(&self, url: &str) -> bool;
}

/// Trait for custom parsers
pub trait CustomParser {
    fn parse(&self, content: &str) -> Result<MarkdownNode, MarkdownProcessorError>;
    fn get_name(&self) -> &str;
}

/// Markdown processor errors
#[derive(Debug, thiserror::Error)]
pub enum MarkdownProcessorError {
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Format error: {0}")]
    FormatError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Text processor error: {0}")]
    TextProcessorError(#[from] TextProcessorError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_markdown_processor_creation() {
        let processor = MarkdownProcessor::new();
        assert_eq!(processor.config.enable_tables, true);
    }

    #[tokio::test]
    async fn test_parse_simple_markdown() {
        let processor = MarkdownProcessor::new();
        let markdown = "# Hello\n\nThis is a **bold** text.";
        let ast = processor.parse(markdown).await.unwrap();
        
        match ast {
            MarkdownNode::Document { children } => {
                assert!(!children.is_empty());
            }
            _ => panic!("Expected document node"),
        }
    }

    #[tokio::test]
    async fn test_validation() {
        let processor = MarkdownProcessor::new();
        let markdown = "# Heading\n\n![](empty-url.jpg)\n\n[Empty link]()";
        let errors = processor.validate(markdown).await.unwrap();
        
        // Should have errors for empty image URL and empty link URL
        assert!(errors.len() >= 2);
    }

    #[tokio::test]
    async fn test_format_detection() {
        let processor = MarkdownProcessor::new();
        let markdown = "This is **bold** and *italic* text.";
        let range = TextRange::new(0, markdown.len());
        let detections = processor.detect_formatting(markdown, range);
        
        assert!(detections.len() >= 2); // Should detect bold and italic
    }

    #[tokio::test]
    async fn test_statistics_generation() {
        let processor = MarkdownProcessor::new();
        let markdown = "# Heading\n\nParagraph with [link](url) and ![image](img.jpg).\n\n## Another heading";
        let stats = processor.generate_statistics(markdown).await.unwrap();
        
        assert_eq!(stats.heading_count.get(&1), Some(&1));
        assert_eq!(stats.heading_count.get(&2), Some(&1));
        assert_eq!(stats.link_count, 1);
        assert_eq!(stats.image_count, 1);
    }

    #[tokio::test]
    async fn test_ast_to_markdown() {
        let processor = MarkdownProcessor::new();
        let original = "# Hello\n\nThis is **bold** text.\n";
        let ast = processor.parse(original).await.unwrap();
        let reconstructed = processor.ast_to_markdown(&ast).unwrap();
        
        // The reconstructed markdown should be functionally equivalent
        assert!(reconstructed.contains("# Hello"));
        assert!(reconstructed.contains("**bold**"));
    }
}