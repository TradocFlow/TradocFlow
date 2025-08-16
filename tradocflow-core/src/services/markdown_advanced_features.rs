use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use pulldown_cmark::{Parser, html, Event, Tag, CodeBlockKind, HeadingLevel};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::markdown_processor::{MarkdownProcessor, TocEntry, LinkInfo, ImageInfo, CodeBlockInfo};
use super::markdown_text_processor::CursorPosition;

/// Advanced features for markdown editor including live preview, syntax highlighting, and export
pub struct MarkdownAdvancedFeatures {
    /// Live preview generator
    preview_generator: LivePreviewGenerator,
    /// Syntax highlighter
    syntax_highlighter: SyntaxHighlighter,
    /// Document outline extractor
    outline_extractor: OutlineExtractor,
    /// Export engine
    export_engine: ExportEngine,
    /// Configuration
    config: AdvancedFeaturesConfig,
}

/// Live preview generator for real-time HTML rendering
pub struct LivePreviewGenerator {
    /// Current HTML cache
    html_cache: Arc<Mutex<HtmlCache>>,
    /// Custom CSS styles
    custom_styles: Vec<String>,
    /// Template engine
    template_engine: TemplateEngine,
    /// Math rendering engine
    math_engine: Option<MathEngine>,
    /// Diagram renderer
    diagram_renderer: Option<DiagramRenderer>,
}

/// HTML cache for performance
#[derive(Debug, Clone)]
struct HtmlCache {
    /// Content hash to HTML mapping
    cache: HashMap<u64, CachedHtml>,
    /// Cache metadata
    metadata: CacheMetadata,
}

/// Cached HTML content
#[derive(Debug, Clone)]
struct CachedHtml {
    /// Generated HTML
    html: String,
    /// Generation timestamp
    timestamp: u64,
    /// Source content hash
    content_hash: u64,
    /// Rendering options used
    options: RenderOptions,
}

/// HTML rendering options
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RenderOptions {
    /// Include table of contents
    pub include_toc: bool,
    /// Enable syntax highlighting
    pub syntax_highlighting: bool,
    /// Enable math rendering
    pub math_rendering: bool,
    /// Enable diagram rendering
    pub diagram_rendering: bool,
    /// Custom CSS classes
    pub custom_css_classes: Vec<String>,
    /// Base URL for relative links
    pub base_url: Option<String>,
    /// Sanitize HTML output
    pub sanitize_html: bool,
    /// Line numbers for code blocks
    pub line_numbers: bool,
}

/// Template engine for HTML generation
#[derive(Debug)]
struct TemplateEngine {
    /// Template cache
    templates: HashMap<String, Template>,
    /// Default template
    default_template: Template,
}

/// HTML template
#[derive(Debug, Clone)]
struct Template {
    /// Template name
    name: String,
    /// Template content
    content: String,
    /// Placeholders
    placeholders: Vec<String>,
}

/// Math rendering engine
#[derive(Debug)]
struct MathEngine {
    /// Math expressions cache
    cache: HashMap<String, String>,
    /// Rendering backend
    backend: MathBackend,
}

/// Math rendering backends
#[derive(Debug, Clone, PartialEq, Eq)]
enum MathBackend {
    KaTeX,
    MathJax,
    None,
}

/// Diagram rendering engine
#[derive(Debug)]
struct DiagramRenderer {
    /// Supported diagram types
    supported_types: Vec<DiagramType>,
    /// Rendering cache
    cache: HashMap<String, RenderedDiagram>,
}

/// Supported diagram types
#[derive(Debug, Clone, PartialEq, Eq)]
enum DiagramType {
    Mermaid,
    PlantUML,
    Graphviz,
    D2,
}

/// Rendered diagram
#[derive(Debug, Clone)]
struct RenderedDiagram {
    /// SVG content
    svg: String,
    /// Source code
    source: String,
    /// Diagram type
    diagram_type: DiagramType,
    /// Rendering timestamp
    timestamp: u64,
}

/// Syntax highlighter for code blocks and inline code
pub struct SyntaxHighlighter {
    /// Language definitions
    languages: HashMap<String, LanguageDefinition>,
    /// Highlighting themes
    themes: HashMap<String, HighlightTheme>,
    /// Current theme
    current_theme: String,
    /// Highlighting cache
    cache: Arc<Mutex<HighlightCache>>,
}

/// Language definition for syntax highlighting
#[derive(Debug, Clone)]
struct LanguageDefinition {
    /// Language name
    name: String,
    /// File extensions
    extensions: Vec<String>,
    /// Syntax patterns
    patterns: Vec<SyntaxPattern>,
    /// Keywords
    keywords: Vec<String>,
    /// Comment styles
    comment_styles: Vec<CommentStyle>,
}

/// Syntax pattern for highlighting
#[derive(Debug, Clone)]
struct SyntaxPattern {
    /// Pattern regex
    pattern: Regex,
    /// Token type
    token_type: TokenType,
    /// CSS class
    css_class: String,
}

/// Types of syntax tokens
#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenType {
    Keyword,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Identifier,
    Type,
    Function,
    Variable,
    Constant,
}

/// Comment styles
#[derive(Debug, Clone)]
struct CommentStyle {
    /// Single line comment prefix
    single_line: Option<String>,
    /// Multi-line comment start
    multi_line_start: Option<String>,
    /// Multi-line comment end
    multi_line_end: Option<String>,
}

/// Highlighting theme
#[derive(Debug, Clone)]
struct HighlightTheme {
    /// Theme name
    name: String,
    /// Token colors
    token_colors: HashMap<TokenType, TokenColor>,
    /// Background color
    background_color: String,
    /// Text color
    text_color: String,
}

/// Token color definition
#[derive(Debug, Clone)]
struct TokenColor {
    /// Foreground color
    foreground: String,
    /// Background color (optional)
    background: Option<String>,
    /// Font weight
    font_weight: Option<String>,
    /// Font style
    font_style: Option<String>,
}

/// Highlighting cache
#[derive(Debug)]
struct HighlightCache {
    /// Code hash to highlighted HTML mapping
    cache: HashMap<u64, HighlightedCode>,
    /// Cache size limit
    max_size: usize,
}

/// Highlighted code result
#[derive(Debug, Clone)]
struct HighlightedCode {
    /// Highlighted HTML
    html: String,
    /// Source code hash
    source_hash: u64,
    /// Language used
    language: String,
    /// Theme used
    theme: String,
    /// Creation timestamp
    timestamp: u64,
}

/// Document outline extractor
pub struct OutlineExtractor {
    /// Outline cache
    cache: Arc<Mutex<OutlineCache>>,
    /// Extraction options
    options: OutlineOptions,
}

/// Outline extraction options
#[derive(Debug, Clone)]
pub struct OutlineOptions {
    /// Maximum depth to extract
    pub max_depth: usize,
    /// Include line numbers
    pub include_line_numbers: bool,
    /// Include word counts
    pub include_word_counts: bool,
    /// Include anchors for navigation
    pub include_anchors: bool,
    /// Custom heading patterns
    pub custom_patterns: Vec<Regex>,
}

/// Document outline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentOutline {
    /// Outline entries
    pub entries: Vec<OutlineEntry>,
    /// Total word count
    pub total_words: usize,
    /// Total character count
    pub total_characters: usize,
    /// Reading time estimate (minutes)
    pub reading_time_minutes: f32,
    /// Document structure statistics
    pub structure_stats: StructureStats,
}

/// Outline entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineEntry {
    /// Entry type
    pub entry_type: OutlineEntryType,
    /// Title/content
    pub title: String,
    /// Heading level (for headings)
    pub level: Option<usize>,
    /// Line number
    pub line_number: usize,
    /// Character position
    pub position: usize,
    /// Word count in section
    pub word_count: usize,
    /// Anchor ID for navigation
    pub anchor: Option<String>,
    /// Child entries
    pub children: Vec<OutlineEntry>,
}

/// Types of outline entries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutlineEntryType {
    Heading,
    Section,
    List,
    Table,
    CodeBlock,
    Image,
    Link,
    Custom(String),
}

/// Document structure statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureStats {
    /// Heading counts by level
    pub heading_counts: Vec<usize>,
    /// Total lists
    pub list_count: usize,
    /// Total tables
    pub table_count: usize,
    /// Total code blocks
    pub code_block_count: usize,
    /// Total images
    pub image_count: usize,
    /// Total links
    pub link_count: usize,
    /// Average words per section
    pub avg_words_per_section: f32,
}

/// Outline cache
#[derive(Debug)]
struct OutlineCache {
    /// Content hash to outline mapping
    cache: HashMap<u64, CachedOutline>,
    /// Cache metadata
    metadata: CacheMetadata,
}

/// Cached outline
#[derive(Debug, Clone)]
struct CachedOutline {
    /// Document outline
    outline: DocumentOutline,
    /// Content hash
    content_hash: u64,
    /// Generation timestamp
    timestamp: u64,
    /// Options used
    options: OutlineOptions,
}

/// Export engine for various output formats
pub struct ExportEngine {
    /// Export templates
    templates: HashMap<ExportFormat, ExportTemplate>,
    /// Export cache
    cache: Arc<Mutex<ExportCache>>,
    /// Export options
    default_options: ExportOptions,
}

/// Export formats
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExportFormat {
    HTML,
    PDF,
    Word,
    LaTeX,
    RTF,
    EPUB,
    Slides,
    WikiText,
    Custom(String),
}

/// Export template
#[derive(Debug, Clone)]
struct ExportTemplate {
    /// Template name
    name: String,
    /// Template content
    content: String,
    /// Required fields
    required_fields: Vec<String>,
    /// Output MIME type
    mime_type: String,
}

/// Export options
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Include table of contents
    pub include_toc: bool,
    /// Include page numbers
    pub include_page_numbers: bool,
    /// Custom CSS/styling
    pub custom_styles: Vec<String>,
    /// Metadata to include
    pub metadata: HashMap<String, String>,
    /// Image handling
    pub image_handling: ImageHandling,
    /// Link handling
    pub link_handling: LinkHandling,
    /// Output quality
    pub quality: ExportQuality,
}

/// Image handling options
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageHandling {
    Embed,
    Link,
    Copy,
    Optimize,
}

/// Link handling options
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkHandling {
    Preserve,
    ConvertToFootnotes,
    RemoveFormatting,
    ValidateAndFix,
}

/// Export quality settings
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportQuality {
    Draft,
    Standard,
    High,
    Print,
}

/// Export cache
#[derive(Debug)]
struct ExportCache {
    /// Export results cache
    cache: HashMap<String, CachedExport>,
    /// Cache size limit
    max_size: usize,
}

/// Cached export result
#[derive(Debug, Clone)]
struct CachedExport {
    /// Export data
    data: Vec<u8>,
    /// Content hash
    content_hash: u64,
    /// Export format
    format: ExportFormat,
    /// Options used
    options: ExportOptions,
    /// Creation timestamp
    timestamp: u64,
    /// File size
    file_size: usize,
}

/// Cache metadata
#[derive(Debug, Clone)]
struct CacheMetadata {
    /// Total entries
    entry_count: usize,
    /// Memory usage
    memory_usage: usize,
    /// Hit count
    hits: u64,
    /// Miss count
    misses: u64,
    /// Last cleanup
    last_cleanup: u64,
}

/// Configuration for advanced features
#[derive(Debug, Clone)]
pub struct AdvancedFeaturesConfig {
    /// Live preview settings
    pub preview: PreviewConfig,
    /// Syntax highlighting settings
    pub syntax: SyntaxConfig,
    /// Outline extraction settings
    pub outline: OutlineOptions,
    /// Export settings
    pub export: ExportOptions,
    /// Performance settings
    pub performance: PerformanceConfig,
}

/// Preview configuration
#[derive(Debug, Clone)]
pub struct PreviewConfig {
    /// Auto-refresh on changes
    pub auto_refresh: bool,
    /// Refresh delay (ms)
    pub refresh_delay_ms: u64,
    /// Enable math rendering
    pub enable_math: bool,
    /// Enable diagram rendering
    pub enable_diagrams: bool,
    /// Custom CSS files
    pub custom_css_files: Vec<PathBuf>,
    /// Template file
    pub template_file: Option<PathBuf>,
}

/// Syntax highlighting configuration
#[derive(Debug, Clone)]
pub struct SyntaxConfig {
    /// Enable syntax highlighting
    pub enabled: bool,
    /// Default theme
    pub default_theme: String,
    /// Cache size
    pub cache_size: usize,
    /// Supported languages
    pub languages: Vec<String>,
    /// Line numbers
    pub line_numbers: bool,
}

/// Performance configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Enable caching
    pub enable_caching: bool,
    /// Cache size limits (MB)
    pub cache_size_mb: usize,
    /// Background processing
    pub background_processing: bool,
    /// Worker thread count
    pub worker_threads: usize,
}

/// Result type for advanced features
pub type AdvancedFeaturesResult<T> = Result<T, AdvancedFeaturesError>;

/// Errors for advanced features
#[derive(Debug, thiserror::Error)]
pub enum AdvancedFeaturesError {
    #[error("Preview generation error: {0}")]
    PreviewError(String),
    #[error("Syntax highlighting error: {0}")]
    SyntaxError(String),
    #[error("Outline extraction error: {0}")]
    OutlineError(String),
    #[error("Export error: {0}")]
    ExportError(String),
    #[error("Template error: {0}")]
    TemplateError(String),
    #[error("Cache error: {0}")]
    CacheError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl MarkdownAdvancedFeatures {
    /// Create new advanced features instance
    pub fn new(config: AdvancedFeaturesConfig) -> AdvancedFeaturesResult<Self> {
        Ok(Self {
            preview_generator: LivePreviewGenerator::new(&config.preview)?,
            syntax_highlighter: SyntaxHighlighter::new(&config.syntax)?,
            outline_extractor: OutlineExtractor::new(config.outline.clone()),
            export_engine: ExportEngine::new(&config.export)?,
            config,
        })
    }

    /// Generate live HTML preview
    pub fn generate_preview(&mut self, processor: &MarkdownProcessor, options: RenderOptions) -> AdvancedFeaturesResult<String> {
        self.preview_generator.generate_html(processor.content(), options)
    }

    /// Highlight syntax for code block
    pub fn highlight_syntax(&mut self, code: &str, language: &str) -> AdvancedFeaturesResult<String> {
        self.syntax_highlighter.highlight(code, language)
    }

    /// Extract document outline
    pub fn extract_outline(&mut self, processor: &MarkdownProcessor) -> AdvancedFeaturesResult<DocumentOutline> {
        self.outline_extractor.extract(processor)
    }

    /// Export document to specified format
    pub fn export_document(&mut self, processor: &MarkdownProcessor, format: ExportFormat, output_path: &Path) -> AdvancedFeaturesResult<()> {
        self.export_engine.export(processor, format, output_path, &self.config.export)
    }

    /// Get available export formats
    pub fn get_export_formats(&self) -> Vec<ExportFormat> {
        self.export_engine.get_supported_formats()
    }

    /// Get available syntax themes
    pub fn get_syntax_themes(&self) -> Vec<String> {
        self.syntax_highlighter.get_available_themes()
    }

    /// Set syntax theme
    pub fn set_syntax_theme(&mut self, theme: &str) -> AdvancedFeaturesResult<()> {
        self.syntax_highlighter.set_theme(theme)
    }

    /// Clear all caches
    pub fn clear_caches(&mut self) {
        self.preview_generator.clear_cache();
        self.syntax_highlighter.clear_cache();
        self.outline_extractor.clear_cache();
        self.export_engine.clear_cache();
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        PerformanceStats {
            preview_cache_size: self.preview_generator.get_cache_size(),
            syntax_cache_size: self.syntax_highlighter.get_cache_size(),
            outline_cache_size: self.outline_extractor.get_cache_size(),
            export_cache_size: self.export_engine.get_cache_size(),
            total_memory_usage: self.get_total_memory_usage(),
        }
    }

    fn get_total_memory_usage(&self) -> usize {
        self.preview_generator.get_cache_size() +
        self.syntax_highlighter.get_cache_size() +
        self.outline_extractor.get_cache_size() +
        self.export_engine.get_cache_size()
    }
}

impl LivePreviewGenerator {
    fn new(config: &PreviewConfig) -> AdvancedFeaturesResult<Self> {
        Ok(Self {
            html_cache: Arc::new(Mutex::new(HtmlCache::new())),
            custom_styles: Vec::new(),
            template_engine: TemplateEngine::new()?,
            math_engine: if config.enable_math {
                Some(MathEngine::new(MathBackend::KaTeX)?)
            } else {
                None
            },
            diagram_renderer: if config.enable_diagrams {
                Some(DiagramRenderer::new()?)
            } else {
                None
            },
        })
    }

    fn generate_html(&mut self, content: &str, options: RenderOptions) -> AdvancedFeaturesResult<String> {
        let content_hash = self.calculate_hash(content);
        
        // Check cache first
        if let Some(cached) = self.get_cached_html(content_hash, &options) {
            return Ok(cached.html);
        }

        // Generate HTML
        let mut html_output = String::new();
        let parser = Parser::new(content);
        
        // Convert markdown to HTML
        html::push_html(&mut html_output, parser);
        
        // Apply post-processing
        if options.math_rendering {
            html_output = self.process_math_expressions(&html_output)?;
        }
        
        if options.diagram_rendering {
            html_output = self.process_diagrams(&html_output)?;
        }
        
        if options.include_toc {
            html_output = self.add_table_of_contents(&html_output)?;
        }
        
        // Apply template
        let final_html = self.template_engine.apply_template(&html_output, &options)?;
        
        // Cache result
        self.cache_html(content_hash, final_html.clone(), options);
        
        Ok(final_html)
    }

    fn get_cached_html(&self, content_hash: u64, options: &RenderOptions) -> Option<CachedHtml> {
        if let Ok(cache) = self.html_cache.lock() {
            if let Some(cached) = cache.cache.get(&content_hash) {
                if cached.options == *options {
                    return Some(cached.clone());
                }
            }
        }
        None
    }

    fn cache_html(&self, content_hash: u64, html: String, options: RenderOptions) {
        if let Ok(mut cache) = self.html_cache.lock() {
            cache.cache.insert(content_hash, CachedHtml {
                html,
                timestamp: current_timestamp(),
                content_hash,
                options,
            });
        }
    }

    fn process_math_expressions(&self, html: &str) -> AdvancedFeaturesResult<String> {
        if let Some(ref math_engine) = self.math_engine {
            math_engine.process(html)
        } else {
            Ok(html.to_string())
        }
    }

    fn process_diagrams(&self, html: &str) -> AdvancedFeaturesResult<String> {
        if let Some(ref diagram_renderer) = self.diagram_renderer {
            diagram_renderer.process(html)
        } else {
            Ok(html.to_string())
        }
    }

    fn add_table_of_contents(&self, html: &str) -> AdvancedFeaturesResult<String> {
        // Extract headings and generate TOC
        let heading_regex = Regex::new(r"<h([1-6])[^>]*>(.*?)</h[1-6]>")
            .map_err(|e| AdvancedFeaturesError::PreviewError(e.to_string()))?;
        
        let mut toc = String::from("<div class=\"table-of-contents\">\n<h2>Table of Contents</h2>\n<ul>\n");
        
        for cap in heading_regex.captures_iter(html) {
            let level = cap[1].parse::<usize>().unwrap_or(1);
            let title = &cap[2];
            let anchor = title.to_lowercase().replace(' ', "-");
            
            toc.push_str(&format!("<li class=\"toc-level-{}\"><a href=\"#{}\">{}</a></li>\n", level, anchor, title));
        }
        
        toc.push_str("</ul>\n</div>\n");
        
        Ok(format!("{}{}", toc, html))
    }

    fn calculate_hash(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    fn clear_cache(&mut self) {
        if let Ok(mut cache) = self.html_cache.lock() {
            cache.cache.clear();
        }
    }

    fn get_cache_size(&self) -> usize {
        if let Ok(cache) = self.html_cache.lock() {
            cache.metadata.memory_usage
        } else {
            0
        }
    }
}

impl SyntaxHighlighter {
    fn new(config: &SyntaxConfig) -> AdvancedFeaturesResult<Self> {
        let mut highlighter = Self {
            languages: HashMap::new(),
            themes: HashMap::new(),
            current_theme: config.default_theme.clone(),
            cache: Arc::new(Mutex::new(HighlightCache::new(config.cache_size))),
        };
        
        highlighter.load_default_languages()?;
        highlighter.load_default_themes()?;
        
        Ok(highlighter)
    }

    fn highlight(&mut self, code: &str, language: &str) -> AdvancedFeaturesResult<String> {
        let code_hash = self.calculate_hash(code);
        
        // Check cache
        if let Some(cached) = self.get_cached_highlight(code_hash, language) {
            return Ok(cached.html);
        }
        
        // Perform highlighting
        let highlighted = self.perform_highlighting(code, language)?;
        
        // Cache result
        self.cache_highlight(code_hash, highlighted.clone(), language);
        
        Ok(highlighted)
    }

    fn perform_highlighting(&self, code: &str, language: &str) -> AdvancedFeaturesResult<String> {
        if let Some(lang_def) = self.languages.get(language) {
            if let Some(theme) = self.themes.get(&self.current_theme) {
                return self.apply_highlighting(code, lang_def, theme);
            }
        }
        
        // Fallback to plain text
        Ok(format!("<pre><code>{}</code></pre>", html_escape(code)))
    }

    fn apply_highlighting(&self, code: &str, lang_def: &LanguageDefinition, theme: &HighlightTheme) -> AdvancedFeaturesResult<String> {
        let mut highlighted = String::new();
        highlighted.push_str(&format!("<pre class=\"language-{}\"><code>", lang_def.name));
        
        // Simple pattern-based highlighting
        let mut processed_code = html_escape(code);
        
        for pattern in &lang_def.patterns {
            let replacement = format!("<span class=\"{}\">${0}</span>", pattern.css_class);
            processed_code = pattern.pattern.replace_all(&processed_code, replacement.as_str()).to_string();
        }
        
        highlighted.push_str(&processed_code);
        highlighted.push_str("</code></pre>");
        
        Ok(highlighted)
    }

    fn load_default_languages(&mut self) -> AdvancedFeaturesResult<()> {
        // Load Rust language definition
        let rust_patterns = vec![
            SyntaxPattern {
                pattern: Regex::new(r"\b(fn|let|mut|const|static|struct|enum|impl|trait|use|mod|pub|crate|super|self|Self)\b")
                    .map_err(|e| AdvancedFeaturesError::SyntaxError(e.to_string()))?,
                token_type: TokenType::Keyword,
                css_class: "keyword".to_string(),
            },
            SyntaxPattern {
                pattern: Regex::new(r#""([^"\\]|\\.)*""#)
                    .map_err(|e| AdvancedFeaturesError::SyntaxError(e.to_string()))?,
                token_type: TokenType::String,
                css_class: "string".to_string(),
            },
            SyntaxPattern {
                pattern: Regex::new(r"//.*$")
                    .map_err(|e| AdvancedFeaturesError::SyntaxError(e.to_string()))?,
                token_type: TokenType::Comment,
                css_class: "comment".to_string(),
            },
        ];
        
        self.languages.insert("rust".to_string(), LanguageDefinition {
            name: "rust".to_string(),
            extensions: vec!["rs".to_string()],
            patterns: rust_patterns,
            keywords: vec!["fn".to_string(), "let".to_string(), "mut".to_string()],
            comment_styles: vec![CommentStyle {
                single_line: Some("//".to_string()),
                multi_line_start: Some("/*".to_string()),
                multi_line_end: Some("*/".to_string()),
            }],
        });
        
        Ok(())
    }

    fn load_default_themes(&mut self) -> AdvancedFeaturesResult<()> {
        let mut default_colors = HashMap::new();
        default_colors.insert(TokenType::Keyword, TokenColor {
            foreground: "#0066cc".to_string(),
            background: None,
            font_weight: Some("bold".to_string()),
            font_style: None,
        });
        default_colors.insert(TokenType::String, TokenColor {
            foreground: "#009900".to_string(),
            background: None,
            font_weight: None,
            font_style: None,
        });
        default_colors.insert(TokenType::Comment, TokenColor {
            foreground: "#999999".to_string(),
            background: None,
            font_weight: None,
            font_style: Some("italic".to_string()),
        });
        
        self.themes.insert("default".to_string(), HighlightTheme {
            name: "default".to_string(),
            token_colors: default_colors,
            background_color: "#ffffff".to_string(),
            text_color: "#000000".to_string(),
        });
        
        Ok(())
    }

    fn get_cached_highlight(&self, code_hash: u64, language: &str) -> Option<HighlightedCode> {
        if let Ok(cache) = self.cache.lock() {
            cache.cache.get(&code_hash).cloned()
        } else {
            None
        }
    }

    fn cache_highlight(&self, code_hash: u64, html: String, language: &str) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.cache.insert(code_hash, HighlightedCode {
                html,
                source_hash: code_hash,
                language: language.to_string(),
                theme: self.current_theme.clone(),
                timestamp: current_timestamp(),
            });
        }
    }

    fn calculate_hash(&self, code: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        code.hash(&mut hasher);
        self.current_theme.hash(&mut hasher);
        hasher.finish()
    }

    fn get_available_themes(&self) -> Vec<String> {
        self.themes.keys().cloned().collect()
    }

    fn set_theme(&mut self, theme: &str) -> AdvancedFeaturesResult<()> {
        if self.themes.contains_key(theme) {
            self.current_theme = theme.to_string();
            Ok(())
        } else {
            Err(AdvancedFeaturesError::SyntaxError(format!("Theme '{}' not found", theme)))
        }
    }

    fn clear_cache(&mut self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.cache.clear();
        }
    }

    fn get_cache_size(&self) -> usize {
        if let Ok(cache) = self.cache.lock() {
            cache.cache.len() * 1024 // Approximate size
        } else {
            0
        }
    }
}

impl OutlineExtractor {
    fn new(options: OutlineOptions) -> Self {
        Self {
            cache: Arc::new(Mutex::new(OutlineCache::new())),
            options,
        }
    }

    fn extract(&mut self, processor: &MarkdownProcessor) -> AdvancedFeaturesResult<DocumentOutline> {
        let content_hash = self.calculate_hash(processor.content());
        
        // Check cache
        if let Some(cached) = self.get_cached_outline(content_hash) {
            return Ok(cached.outline);
        }
        
        // Generate outline
        let outline = self.generate_outline(processor)?;
        
        // Cache result
        self.cache_outline(content_hash, outline.clone());
        
        Ok(outline)
    }

    fn generate_outline(&self, processor: &MarkdownProcessor) -> AdvancedFeaturesResult<DocumentOutline> {
        let content = processor.content();
        let mut entries = Vec::new();
        let mut structure_stats = StructureStats {
            heading_counts: vec![0; 6],
            list_count: 0,
            table_count: 0,
            code_block_count: 0,
            image_count: 0,
            link_count: 0,
            avg_words_per_section: 0.0,
        };

        // Parse content line by line
        let lines: Vec<&str> = content.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            let line_content = line.trim();
            
            // Check for headings
            if let Some(caps) = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap().captures(line_content) {
                let level = caps[1].len();
                let title = caps[2].to_string();
                let position = content[..content.find(line).unwrap_or(0)].len();
                
                structure_stats.heading_counts[level - 1] += 1;
                
                entries.push(OutlineEntry {
                    entry_type: OutlineEntryType::Heading,
                    title,
                    level: Some(level),
                    line_number: line_num + 1,
                    position,
                    word_count: line_content.split_whitespace().count(),
                    anchor: if self.options.include_anchors {
                        Some(format!("heading-{}", entries.len()))
                    } else {
                        None
                    },
                    children: Vec::new(),
                });
            }
            
            // Check for lists
            if Regex::new(r"^\s*[-*+]\s+").unwrap().is_match(line_content) ||
               Regex::new(r"^\s*\d+\.\s+").unwrap().is_match(line_content) {
                structure_stats.list_count += 1;
            }
            
            // Check for code blocks
            if line_content.starts_with("```") {
                structure_stats.code_block_count += 1;
            }
            
            // Check for images
            if Regex::new(r"!\[.*?\]\(.*?\)").unwrap().is_match(line_content) {
                structure_stats.image_count += 1;
            }
            
            // Check for links
            structure_stats.link_count += Regex::new(r"\[.*?\]\(.*?\)").unwrap().find_iter(line_content).count();
        }

        let total_words = content.split_whitespace().count();
        let total_characters = content.chars().count();
        let reading_time_minutes = total_words as f32 / 225.0; // Average reading speed

        if !entries.is_empty() {
            structure_stats.avg_words_per_section = total_words as f32 / entries.len() as f32;
        }

        Ok(DocumentOutline {
            entries,
            total_words,
            total_characters,
            reading_time_minutes,
            structure_stats,
        })
    }

    fn get_cached_outline(&self, content_hash: u64) -> Option<CachedOutline> {
        if let Ok(cache) = self.cache.lock() {
            cache.cache.get(&content_hash).cloned()
        } else {
            None
        }
    }

    fn cache_outline(&self, content_hash: u64, outline: DocumentOutline) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.cache.insert(content_hash, CachedOutline {
                outline,
                content_hash,
                timestamp: current_timestamp(),
                options: self.options.clone(),
            });
        }
    }

    fn calculate_hash(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    fn clear_cache(&mut self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.cache.clear();
        }
    }

    fn get_cache_size(&self) -> usize {
        if let Ok(cache) = self.cache.lock() {
            cache.metadata.memory_usage
        } else {
            0
        }
    }
}

impl ExportEngine {
    fn new(options: &ExportOptions) -> AdvancedFeaturesResult<Self> {
        let mut engine = Self {
            templates: HashMap::new(),
            cache: Arc::new(Mutex::new(ExportCache::new())),
            default_options: options.clone(),
        };
        
        engine.load_default_templates()?;
        Ok(engine)
    }

    fn export(&mut self, processor: &MarkdownProcessor, format: ExportFormat, output_path: &Path, options: &ExportOptions) -> AdvancedFeaturesResult<()> {
        let content_hash = self.calculate_hash(processor.content());
        let cache_key = format!("{:?}_{}", format, content_hash);
        
        // Check cache
        if let Some(cached) = self.get_cached_export(&cache_key) {
            return std::fs::write(output_path, &cached.data)
                .map_err(AdvancedFeaturesError::IoError);
        }
        
        // Generate export
        let export_data = self.generate_export(processor, &format, options)?;
        
        // Cache result
        self.cache_export(cache_key, export_data.clone(), format.clone(), options.clone(), content_hash);
        
        // Write to file
        std::fs::write(output_path, export_data)
            .map_err(AdvancedFeaturesError::IoError)
    }

    fn generate_export(&self, processor: &MarkdownProcessor, format: &ExportFormat, options: &ExportOptions) -> AdvancedFeaturesResult<Vec<u8>> {
        match format {
            ExportFormat::HTML => {
                let mut html_output = String::new();
                let parser = Parser::new(processor.content());
                html::push_html(&mut html_output, parser);
                
                if options.include_toc {
                    html_output = self.add_toc_to_html(&html_output)?;
                }
                
                Ok(html_output.into_bytes())
            }
            ExportFormat::PDF => {
                // This would require a PDF generation library
                Err(AdvancedFeaturesError::ExportError("PDF export not yet implemented".to_string()))
            }
            _ => Err(AdvancedFeaturesError::ExportError(format!("Export format {:?} not supported", format)))
        }
    }

    fn add_toc_to_html(&self, html: &str) -> AdvancedFeaturesResult<String> {
        // Simple TOC generation
        Ok(format!("<div class=\"toc\">Table of Contents</div>\n{}", html))
    }

    fn load_default_templates(&mut self) -> AdvancedFeaturesResult<()> {
        // Load default HTML template
        self.templates.insert(ExportFormat::HTML, ExportTemplate {
            name: "default_html".to_string(),
            content: r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{{title}}</title>
    <style>{{styles}}</style>
</head>
<body>
    {{content}}
</body>
</html>"#.to_string(),
            required_fields: vec!["title".to_string(), "content".to_string()],
            mime_type: "text/html".to_string(),
        });
        
        Ok(())
    }

    fn get_supported_formats(&self) -> Vec<ExportFormat> {
        self.templates.keys().cloned().collect()
    }

    fn get_cached_export(&self, cache_key: &str) -> Option<CachedExport> {
        if let Ok(cache) = self.cache.lock() {
            cache.cache.get(cache_key).cloned()
        } else {
            None
        }
    }

    fn cache_export(&self, cache_key: String, data: Vec<u8>, format: ExportFormat, options: ExportOptions, content_hash: u64) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.cache.insert(cache_key, CachedExport {
                data,
                content_hash,
                format,
                options,
                timestamp: current_timestamp(),
                file_size: data.len(),
            });
        }
    }

    fn calculate_hash(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    fn clear_cache(&mut self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.cache.clear();
        }
    }

    fn get_cache_size(&self) -> usize {
        if let Ok(cache) = self.cache.lock() {
            cache.cache.values().map(|c| c.file_size).sum()
        } else {
            0
        }
    }
}

/// Performance statistics for advanced features
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub preview_cache_size: usize,
    pub syntax_cache_size: usize,
    pub outline_cache_size: usize,
    pub export_cache_size: usize,
    pub total_memory_usage: usize,
}

// Helper implementations

impl HtmlCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            metadata: CacheMetadata {
                entry_count: 0,
                memory_usage: 0,
                hits: 0,
                misses: 0,
                last_cleanup: current_timestamp(),
            },
        }
    }
}

impl HighlightCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
        }
    }
}

impl OutlineCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            metadata: CacheMetadata {
                entry_count: 0,
                memory_usage: 0,
                hits: 0,
                misses: 0,
                last_cleanup: current_timestamp(),
            },
        }
    }
}

impl ExportCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_size: 100, // Default max entries
        }
    }
}

impl TemplateEngine {
    fn new() -> AdvancedFeaturesResult<Self> {
        Ok(Self {
            templates: HashMap::new(),
            default_template: Template {
                name: "default".to_string(),
                content: "{{content}}".to_string(),
                placeholders: vec!["content".to_string()],
            },
        })
    }

    fn apply_template(&self, content: &str, options: &RenderOptions) -> AdvancedFeaturesResult<String> {
        let template = &self.default_template;
        let mut result = template.content.clone();
        result = result.replace("{{content}}", content);
        Ok(result)
    }
}

impl MathEngine {
    fn new(backend: MathBackend) -> AdvancedFeaturesResult<Self> {
        Ok(Self {
            cache: HashMap::new(),
            backend,
        })
    }

    fn process(&self, html: &str) -> AdvancedFeaturesResult<String> {
        // Placeholder for math processing
        Ok(html.to_string())
    }
}

impl DiagramRenderer {
    fn new() -> AdvancedFeaturesResult<Self> {
        Ok(Self {
            supported_types: vec![DiagramType::Mermaid, DiagramType::PlantUML],
            cache: HashMap::new(),
        })
    }

    fn process(&self, html: &str) -> AdvancedFeaturesResult<String> {
        // Placeholder for diagram processing
        Ok(html.to_string())
    }
}

impl Default for AdvancedFeaturesConfig {
    fn default() -> Self {
        Self {
            preview: PreviewConfig {
                auto_refresh: true,
                refresh_delay_ms: 500,
                enable_math: true,
                enable_diagrams: true,
                custom_css_files: Vec::new(),
                template_file: None,
            },
            syntax: SyntaxConfig {
                enabled: true,
                default_theme: "default".to_string(),
                cache_size: 1000,
                languages: vec!["rust".to_string(), "javascript".to_string(), "python".to_string()],
                line_numbers: true,
            },
            outline: OutlineOptions {
                max_depth: 6,
                include_line_numbers: true,
                include_word_counts: true,
                include_anchors: true,
                custom_patterns: Vec::new(),
            },
            export: ExportOptions {
                include_toc: true,
                include_page_numbers: true,
                custom_styles: Vec::new(),
                metadata: HashMap::new(),
                image_handling: ImageHandling::Embed,
                link_handling: LinkHandling::Preserve,
                quality: ExportQuality::Standard,
            },
            performance: PerformanceConfig {
                enable_caching: true,
                cache_size_mb: 50,
                background_processing: true,
                worker_threads: 2,
            },
        }
    }
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            include_toc: false,
            syntax_highlighting: true,
            math_rendering: false,
            diagram_rendering: false,
            custom_css_classes: Vec::new(),
            base_url: None,
            sanitize_html: true,
            line_numbers: false,
        }
    }
}

/// HTML escape utility function
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Get current timestamp in milliseconds since Unix epoch
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_features_creation() {
        let config = AdvancedFeaturesConfig::default();
        let features = MarkdownAdvancedFeatures::new(config);
        assert!(features.is_ok());
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("Hello <world>"), "Hello &lt;world&gt;");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_render_options_default() {
        let options = RenderOptions::default();
        assert!(!options.include_toc);
        assert!(options.syntax_highlighting);
        assert!(options.sanitize_html);
    }

    #[test]
    fn test_outline_entry_creation() {
        let entry = OutlineEntry {
            entry_type: OutlineEntryType::Heading,
            title: "Test Heading".to_string(),
            level: Some(1),
            line_number: 1,
            position: 0,
            word_count: 2,
            anchor: Some("test-heading".to_string()),
            children: Vec::new(),
        };
        
        assert_eq!(entry.title, "Test Heading");
        assert_eq!(entry.level, Some(1));
    }
}