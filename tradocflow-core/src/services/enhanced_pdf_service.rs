use crate::{Result, TradocumentError};
use pulldown_cmark::{Parser, Event, Tag, HeadingLevel, CodeBlockKind, CowStr, LinkType, Alignment};
use genpdf::{Document, Element, style::Style, elements, Margins, RenderResult, Position};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::fs;

/// Enhanced PDF export configuration with comprehensive formatting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedPdfConfig {
    /// Paper size and margins
    pub paper_size: PaperSize,
    pub margins: MarginSettings,
    
    /// Typography settings
    pub font_config: FontConfiguration,
    
    /// Content formatting options
    pub formatting: FormattingOptions,
    
    /// Advanced options
    pub advanced: AdvancedOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSize {
    pub format: PaperFormat,
    pub orientation: Orientation,
    pub custom_width_mm: Option<f64>,
    pub custom_height_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaperFormat {
    A4,
    Letter,
    Legal,
    A3,
    A5,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginSettings {
    pub top_mm: f64,
    pub bottom_mm: f64,
    pub left_mm: f64,
    pub right_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfiguration {
    pub base_font: String,
    pub base_font_size: f64,
    pub heading_font: String,
    pub code_font: String,
    pub line_height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingOptions {
    pub include_table_of_contents: bool,
    pub include_page_numbers: bool,
    pub include_headers_footers: bool,
    pub header_text: Option<String>,
    pub footer_text: Option<String>,
    pub syntax_highlighting: bool,
    pub preserve_code_formatting: bool,
    pub table_styling: TableStyling,
    pub link_handling: LinkHandling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedOptions {
    pub image_quality: ImageQuality,
    pub compression_level: CompressionLevel,
    pub watermark: Option<WatermarkConfig>,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStyling {
    pub border_width: f64,
    pub border_color: String,
    pub header_background: Option<String>,
    pub alternating_rows: bool,
    pub padding: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkHandling {
    Preserve,
    RemoveFormatting,
    ConvertToFootnotes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageQuality {
    Low,
    Medium,
    High,
    Original,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionLevel {
    None,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkConfig {
    pub text: String,
    pub opacity: f64,
    pub position: WatermarkPosition,
    pub font_size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatermarkPosition {
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Vec<String>,
    pub creator: String,
}

/// Progress information for PDF generation
#[derive(Debug, Clone)]
pub struct PdfExportProgress {
    pub stage: PdfExportStage,
    pub progress_percent: f32,
    pub current_item: String,
    pub items_completed: usize,
    pub total_items: usize,
    pub message: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PdfExportStage {
    Initializing,
    ParsingMarkdown,
    ProcessingImages,
    GeneratingToc,
    RenderingContent,
    ApplyingStyles,
    FinalizeDocument,
    Completed,
    Failed(String),
}

impl std::fmt::Display for PdfExportStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfExportStage::Initializing => write!(f, "Initializing"),
            PdfExportStage::ParsingMarkdown => write!(f, "Parsing Markdown"),
            PdfExportStage::ProcessingImages => write!(f, "Processing Images"),
            PdfExportStage::GeneratingToc => write!(f, "Generating Table of Contents"),
            PdfExportStage::RenderingContent => write!(f, "Rendering Content"),
            PdfExportStage::ApplyingStyles => write!(f, "Applying Styles"),
            PdfExportStage::FinalizeDocument => write!(f, "Finalizing Document"),
            PdfExportStage::Completed => write!(f, "Completed"),
            PdfExportStage::Failed(err) => write!(f, "Failed: {}", err),
        }
    }
}

type ProgressCallback = Arc<dyn Fn(PdfExportProgress) + Send + Sync>;

/// Enhanced PDF generation service with professional formatting
pub struct EnhancedPdfService {
    config: EnhancedPdfConfig,
    progress_sender: Option<mpsc::UnboundedSender<PdfExportProgress>>,
    warnings: Vec<String>,
}

impl EnhancedPdfService {
    pub fn new(config: EnhancedPdfConfig) -> Self {
        Self {
            config,
            progress_sender: None,
            warnings: Vec::new(),
        }
    }
    
    pub fn with_progress_channel(&mut self) -> mpsc::UnboundedReceiver<PdfExportProgress> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.progress_sender = Some(sender);
        receiver
    }
    
    /// Generate PDF from markdown content with comprehensive formatting
    pub async fn export_markdown_to_pdf(
        &mut self,
        markdown_content: &str,
        output_path: &std::path::Path,
    ) -> Result<PdfExportResult> {
        let start_time = std::time::Instant::now();
        
        // Initialize progress tracking
        self.send_progress(PdfExportProgress {
            stage: PdfExportStage::Initializing,
            progress_percent: 0.0,
            current_item: "Setting up PDF generation".to_string(),
            items_completed: 0,
            total_items: 6,
            message: "Preparing PDF export configuration...".to_string(),
            warnings: Vec::new(),
        });
        
        // Load and validate fonts
        let font_family = match self.load_fonts() {
            Ok(fonts) => fonts,
            Err(e) => {
                let error_msg = format!("Font loading failed: {}", e);
                self.send_progress(PdfExportProgress {
                    stage: PdfExportStage::Failed(error_msg.clone()),
                    progress_percent: 0.0,
                    current_item: "Font loading".to_string(),
                    items_completed: 0,
                    total_items: 6,
                    message: error_msg.clone(),
                    warnings: self.warnings.clone(),
                });
                return Err(TradocumentError::Pdf(error_msg));
            }
        };
        
        // Create document with proper configuration
        let mut doc = self.create_document_with_config(font_family)?;
        
        // Parse markdown with progress tracking
        self.send_progress(PdfExportProgress {
            stage: PdfExportStage::ParsingMarkdown,
            progress_percent: 16.7,
            current_item: "Parsing markdown content".to_string(),
            items_completed: 1,
            total_items: 6,
            message: format!("Processing {} characters of markdown", markdown_content.len()),
            warnings: self.warnings.clone(),
        });
        
        let parsed_content = self.parse_markdown_with_enhancements(markdown_content)?;
        
        // Generate table of contents if requested
        if self.config.formatting.include_table_of_contents {
            self.send_progress(PdfExportProgress {
                stage: PdfExportStage::GeneratingToc,
                progress_percent: 33.3,
                current_item: "Generating table of contents".to_string(),
                items_completed: 2,
                total_items: 6,
                message: format!("Found {} headings", parsed_content.headings.len()),
                warnings: self.warnings.clone(),
            });
            
            self.add_table_of_contents(&mut doc, &parsed_content.headings)?;
        }
        
        // Process and render content
        self.send_progress(PdfExportProgress {
            stage: PdfExportStage::RenderingContent,
            progress_percent: 50.0,
            current_item: "Rendering document content".to_string(),
            items_completed: 3,
            total_items: 6,
            message: format!("Rendering {} content blocks", parsed_content.elements.len()),
            warnings: self.warnings.clone(),
        });
        
        self.render_content_to_document(&mut doc, &parsed_content)?;
        
        // Apply advanced formatting
        self.send_progress(PdfExportProgress {
            stage: PdfExportStage::ApplyingStyles,
            progress_percent: 66.7,
            current_item: "Applying advanced formatting".to_string(),
            items_completed: 4,
            total_items: 6,
            message: "Adding headers, footers, and page numbers".to_string(),
            warnings: self.warnings.clone(),
        });
        
        self.apply_advanced_formatting(&mut doc)?;
        
        // Finalize and render to file
        self.send_progress(PdfExportProgress {
            stage: PdfExportStage::FinalizeDocument,
            progress_percent: 83.3,
            current_item: "Writing PDF file".to_string(),
            items_completed: 5,
            total_items: 6,
            message: format!("Writing to {}", output_path.display()),
            warnings: self.warnings.clone(),
        });
        
        // Render the document to bytes first to get file size
        let mut pdf_bytes = Vec::new();
        doc.render(&mut pdf_bytes)
            .map_err(|e| TradocumentError::Pdf(format!("PDF rendering failed: {}", e)))?;
        
        // Write to file
        fs::write(output_path, &pdf_bytes)
            .map_err(|e| TradocumentError::Pdf(format!("Failed to write PDF file: {}", e)))?;
        
        let export_time = start_time.elapsed();
        
        // Complete
        self.send_progress(PdfExportProgress {
            stage: PdfExportStage::Completed,
            progress_percent: 100.0,
            current_item: "Export completed".to_string(),
            items_completed: 6,
            total_items: 6,
            message: format!("PDF exported successfully in {:.2}s", export_time.as_secs_f64()),
            warnings: self.warnings.clone(),
        });
        
        Ok(PdfExportResult {
            file_path: output_path.to_path_buf(),
            file_size_bytes: pdf_bytes.len() as u64,
            generation_time_ms: export_time.as_millis() as u64,
            page_count: 0, // TODO: Calculate actual page count
            warnings: self.warnings.clone(),
            content_stats: ContentStats {
                headings: parsed_content.headings.len(),
                paragraphs: parsed_content.paragraphs,
                code_blocks: parsed_content.code_blocks,
                tables: parsed_content.tables,
                images: parsed_content.images,
                links: parsed_content.links,
            },
        })
    }
    
    fn send_progress(&self, progress: PdfExportProgress) {
        if let Some(sender) = &self.progress_sender {
            let _ = sender.send(progress);
        }
    }
    
    fn load_fonts(&mut self) -> Result<genpdf::fonts::FontFamily<genpdf::fonts::FontData>> {
        // Try to load fonts from system or bundled fonts
        let font_paths = vec![
            "fonts",
            "/usr/share/fonts/truetype/liberation",
            "/System/Library/Fonts",
            "/Windows/Fonts",
        ];
        
        for font_path in font_paths {
            if let Ok(font_family) = genpdf::fonts::from_files(
                font_path, 
                &self.config.font_config.base_font, 
                None
            ) {
                return Ok(font_family);
            }
        }
        
        // Fallback to default system fonts
        self.warnings.push("Using fallback fonts - some formatting may be limited".to_string());
        
        genpdf::fonts::from_files(".", "LiberationSans", None)
            .or_else(|_| genpdf::fonts::from_files(".", "Arial", None))
            .or_else(|_| genpdf::fonts::from_files(".", "DejaVuSans", None))
            .map_err(|e| TradocumentError::Pdf(format!("No suitable fonts found: {}", e)))
    }
    
    fn create_document_with_config(
        &self, 
        font_family: genpdf::fonts::FontFamily<genpdf::fonts::FontData>
    ) -> Result<Document> {
        let mut doc = Document::new(font_family);
        
        // Set document metadata
        if let Some(title) = &self.config.advanced.metadata.title {
            doc.set_title(title);
        }
        
        // Set margins
        let margins = Margins::trbl(
            self.config.margins.top_mm,
            self.config.margins.right_mm,
            self.config.margins.bottom_mm,
            self.config.margins.left_mm,
        );
        doc.set_margins(margins);
        
        // Set paper size
        let size = match self.config.paper_size.format {
            PaperFormat::A4 => genpdf::Size::A4,
            PaperFormat::Letter => genpdf::Size::Letter,
            PaperFormat::Legal => genpdf::Size::Legal,
            PaperFormat::A3 => genpdf::Size::A3,
            PaperFormat::A5 => genpdf::Size::A5,
            PaperFormat::Custom => {
                if let (Some(width), Some(height)) = (
                    self.config.paper_size.custom_width_mm,
                    self.config.paper_size.custom_height_mm,
                ) {
                    genpdf::Size::new(width, height)
                } else {
                    genpdf::Size::A4 // Fallback
                }
            }
        };
        
        doc.set_paper_size(size);
        
        Ok(doc)
    }
    
    fn parse_markdown_with_enhancements(&mut self, content: &str) -> Result<ParsedContent> {
        let parser = Parser::new(content);
        let mut parsed = ParsedContent::new();
        let mut current_text = String::new();
        let mut in_heading = false;
        let mut heading_level = 1;
        let mut in_code_block = false;
        let mut code_language = None;
        let mut in_table = false;
        let mut table_headers: Vec<String> = Vec::new();
        let mut table_alignments: Vec<Alignment> = Vec::new();
        let mut table_rows: Vec<Vec<String>> = Vec::new();
        let mut current_table_row: Vec<String> = Vec::new();
        
        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, id, classes, attrs }) => {
                    in_heading = true;
                    heading_level = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                }
                Event::End(Tag::Heading { .. }) if in_heading => {
                    if !current_text.is_empty() {
                        parsed.headings.push(HeadingInfo {
                            level: heading_level,
                            title: current_text.trim().to_string(),
                            page_number: None, // Will be filled during rendering
                        });
                        parsed.elements.push(ContentElement::Heading {
                            level: heading_level,
                            text: current_text.trim().to_string(),
                        });
                        current_text.clear();
                    }
                    in_heading = false;
                }
                Event::Start(Tag::Paragraph) => {
                    // Start of paragraph
                }
                Event::End(Tag::Paragraph) if !in_heading && !in_code_block => {
                    if !current_text.is_empty() {
                        parsed.paragraphs += 1;
                        parsed.elements.push(ContentElement::Paragraph {
                            text: current_text.trim().to_string(),
                        });
                        current_text.clear();
                    }
                }
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                    in_code_block = true;
                    code_language = if lang.is_empty() { None } else { Some(lang.to_string()) };
                }
                Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                    in_code_block = true;
                    code_language = None;
                }
                Event::End(Tag::CodeBlock(_)) if in_code_block => {
                    if !current_text.is_empty() {
                        parsed.code_blocks += 1;
                        parsed.elements.push(ContentElement::CodeBlock {
                            language: code_language.clone(),
                            code: current_text.clone(),
                        });
                        current_text.clear();
                    }
                    in_code_block = false;
                    code_language = None;
                }
                Event::Start(Tag::Table(alignments)) => {
                    in_table = true;
                    table_alignments = alignments;
                    table_headers.clear();
                    table_rows.clear();
                }
                Event::Start(Tag::TableHead) => {
                    // Starting table header
                }
                Event::End(Tag::TableHead) => {
                    // End of table header
                }
                Event::Start(Tag::TableRow) => {
                    current_table_row.clear();
                }
                Event::End(Tag::TableRow) => {
                    if in_table {
                        if table_headers.is_empty() {
                            table_headers = current_table_row.clone();
                        } else {
                            table_rows.push(current_table_row.clone());
                        }
                        current_table_row.clear();
                    }
                }
                Event::Start(Tag::TableCell) => {
                    current_text.clear();
                }
                Event::End(Tag::TableCell) => {
                    current_table_row.push(current_text.trim().to_string());
                    current_text.clear();
                }
                Event::End(Tag::Table(_)) if in_table => {
                    parsed.tables += 1;
                    parsed.elements.push(ContentElement::Table {
                        headers: table_headers.clone(),
                        rows: table_rows.clone(),
                        alignments: table_alignments.clone(),
                    });
                    in_table = false;
                }
                Event::Start(Tag::List(start_number)) => {
                    parsed.elements.push(ContentElement::ListStart {
                        ordered: start_number.is_some(),
                        start_number,
                    });
                }
                Event::End(Tag::List(_)) => {
                    parsed.elements.push(ContentElement::ListEnd);
                }
                Event::Start(Tag::Item) => {
                    // List item start
                }
                Event::End(Tag::Item) => {
                    if !current_text.is_empty() {
                        parsed.elements.push(ContentElement::ListItem {
                            text: current_text.trim().to_string(),
                        });
                        current_text.clear();
                    }
                }
                Event::Start(Tag::Link(LinkType::Inline, url, title)) => {
                    parsed.links += 1;
                    // Handle link based on configuration
                    match self.config.formatting.link_handling {
                        LinkHandling::Preserve => {
                            // Keep link formatting
                        }
                        LinkHandling::RemoveFormatting => {
                            // Will just render as text
                        }
                        LinkHandling::ConvertToFootnotes => {
                            // Add as footnote (TODO: implement footnote system)
                        }
                    }
                }
                Event::Start(Tag::Image(LinkType::Inline, url, title)) => {
                    parsed.images += 1;
                    parsed.elements.push(ContentElement::Image {
                        url: url.to_string(),
                        alt_text: current_text.clone(),
                        title: if title.is_empty() { None } else { Some(title.to_string()) },
                    });
                    current_text.clear();
                }
                Event::Text(text) => {
                    current_text.push_str(&text);
                }
                Event::Code(code) => {
                    current_text.push_str(&format!("`{}`", code));
                }
                Event::SoftBreak | Event::HardBreak => {
                    if in_code_block {
                        current_text.push('\n');
                    } else {
                        current_text.push(' ');
                    }
                }
                Event::Html(html) => {
                    // Basic HTML handling (strip tags for plain text)
                    self.warnings.push("HTML content was simplified for PDF".to_string());
                }
                _ => {
                    // Handle other markdown events as needed
                }
            }
        }
        
        // Add any remaining text
        if !current_text.is_empty() && !in_heading {
            parsed.elements.push(ContentElement::Paragraph {
                text: current_text.trim().to_string(),
            });
            parsed.paragraphs += 1;
        }
        
        Ok(parsed)
    }
    
    fn add_table_of_contents(&self, doc: &mut Document, headings: &[HeadingInfo]) -> Result<()> {
        if headings.is_empty() {
            return Ok(());
        }
        
        // Add TOC title
        let toc_style = Style::new()
            .bold()
            .with_font_size(16.0);
        doc.push(elements::Paragraph::new("Table of Contents").styled(toc_style));
        doc.push(elements::Break::new(0.5));
        
        // Add TOC entries
        for heading in headings {
            let indent = "  ".repeat((heading.level - 1) as usize);
            let toc_entry = format!("{}{}", indent, heading.title);
            let toc_entry_style = Style::new().with_font_size(
                12.0 - (heading.level as f64 * 0.5)
            );
            doc.push(elements::Paragraph::new(toc_entry).styled(toc_entry_style));
        }
        
        doc.push(elements::Break::new(1.0));
        Ok(())
    }
    
    fn render_content_to_document(&self, doc: &mut Document, content: &ParsedContent) -> Result<()> {
        let mut list_depth = 0;
        let mut list_numbers: Vec<usize> = Vec::new();
        
        for element in &content.elements {
            match element {
                ContentElement::Heading { level, text } => {
                    let size = match level {
                        1 => self.config.font_config.base_font_size + 6.0,
                        2 => self.config.font_config.base_font_size + 4.0,
                        3 => self.config.font_config.base_font_size + 2.0,
                        4 => self.config.font_config.base_font_size + 1.0,
                        _ => self.config.font_config.base_font_size,
                    };
                    
                    let heading_style = Style::new()
                        .bold()
                        .with_font_size(size);
                    
                    doc.push(elements::Break::new(0.5));
                    doc.push(elements::Paragraph::new(text).styled(heading_style));
                    doc.push(elements::Break::new(0.3));
                }
                ContentElement::Paragraph { text } => {
                    let para_style = Style::new()
                        .with_font_size(self.config.font_config.base_font_size);
                    doc.push(elements::Paragraph::new(text).styled(para_style));
                    doc.push(elements::Break::new(0.2));
                }
                ContentElement::CodeBlock { language: _, code } => {
                    let code_style = Style::new()
                        .with_font_size(self.config.font_config.base_font_size - 1.0);
                    
                    // Create a framed code block
                    let code_element = elements::Paragraph::new(code)
                        .styled(code_style)
                        .framed();
                    
                    doc.push(code_element);
                    doc.push(elements::Break::new(0.3));
                }
                ContentElement::Table { headers, rows, alignments: _ } => {
                    self.render_table(doc, headers, rows)?;
                    doc.push(elements::Break::new(0.3));
                }
                ContentElement::ListStart { ordered, start_number } => {
                    list_depth += 1;
                    if *ordered {
                        list_numbers.push(start_number.unwrap_or(1));
                    } else {
                        list_numbers.push(0); // 0 indicates unordered list
                    }
                }
                ContentElement::ListItem { text } => {
                    let indent = "  ".repeat(list_depth.saturating_sub(1));
                    let bullet = if list_numbers.last() == Some(&0) {
                        "•".to_string()
                    } else if let Some(num) = list_numbers.last_mut() {
                        let current = *num;
                        *num += 1;
                        format!("{}.", current)
                    } else {
                        "•".to_string()
                    };
                    
                    let list_item_text = format!("{}{} {}", indent, bullet, text);
                    let list_style = Style::new()
                        .with_font_size(self.config.font_config.base_font_size);
                    
                    doc.push(elements::Paragraph::new(list_item_text).styled(list_style));
                }
                ContentElement::ListEnd => {
                    list_depth = list_depth.saturating_sub(1);
                    list_numbers.pop();
                    doc.push(elements::Break::new(0.2));
                }
                ContentElement::Image { url: _, alt_text, title: _ } => {
                    // For now, just add alt text as a placeholder
                    // TODO: Implement actual image loading and embedding
                    let image_placeholder = format!("[Image: {}]", alt_text);
                    let image_style = Style::new()
                        .italic()
                        .with_font_size(self.config.font_config.base_font_size - 1.0);
                    
                    doc.push(elements::Paragraph::new(image_placeholder).styled(image_style));
                    doc.push(elements::Break::new(0.2));
                }
            }
        }
        
        Ok(())
    }
    
    fn render_table(&self, doc: &mut Document, headers: &[String], rows: &[Vec<String>]) -> Result<()> {
        // Create table header
        if !headers.is_empty() {
            let header_text = headers.join(" | ");
            let header_style = Style::new()
                .bold()
                .with_font_size(self.config.font_config.base_font_size);
            
            doc.push(elements::Paragraph::new(header_text).styled(header_style));
            
            // Add separator
            let separator = "-".repeat(headers.iter().map(|h| h.len() + 3).sum::<usize>());
            doc.push(elements::Paragraph::new(separator));
        }
        
        // Create table rows
        for row in rows {
            let row_text = row.join(" | ");
            let row_style = Style::new()
                .with_font_size(self.config.font_config.base_font_size);
            
            doc.push(elements::Paragraph::new(row_text).styled(row_style));
        }
        
        Ok(())
    }
    
    fn apply_advanced_formatting(&self, _doc: &mut Document) -> Result<()> {
        // TODO: Implement headers, footers, page numbers, watermarks
        // This would require more advanced genpdf features or a different PDF library
        Ok(())
    }
}

impl Default for EnhancedPdfConfig {
    fn default() -> Self {
        Self {
            paper_size: PaperSize {
                format: PaperFormat::A4,
                orientation: Orientation::Portrait,
                custom_width_mm: None,
                custom_height_mm: None,
            },
            margins: MarginSettings {
                top_mm: 25.0,
                bottom_mm: 25.0,
                left_mm: 25.0,
                right_mm: 25.0,
            },
            font_config: FontConfiguration {
                base_font: "LiberationSans".to_string(),
                base_font_size: 11.0,
                heading_font: "LiberationSans".to_string(),
                code_font: "LiberationMono".to_string(),
                line_height: 1.4,
            },
            formatting: FormattingOptions {
                include_table_of_contents: true,
                include_page_numbers: true,
                include_headers_footers: false,
                header_text: None,
                footer_text: None,
                syntax_highlighting: false,
                preserve_code_formatting: true,
                table_styling: TableStyling {
                    border_width: 1.0,
                    border_color: "#000000".to_string(),
                    header_background: Some("#f5f5f5".to_string()),
                    alternating_rows: true,
                    padding: 5.0,
                },
                link_handling: LinkHandling::Preserve,
            },
            advanced: AdvancedOptions {
                image_quality: ImageQuality::Medium,
                compression_level: CompressionLevel::Medium,
                watermark: None,
                metadata: DocumentMetadata {
                    title: None,
                    author: None,
                    subject: None,
                    keywords: Vec::new(),
                    creator: "TradocFlow Core".to_string(),
                },
            },
        }
    }
}

#[derive(Debug)]
struct ParsedContent {
    elements: Vec<ContentElement>,
    headings: Vec<HeadingInfo>,
    paragraphs: usize,
    code_blocks: usize,
    tables: usize,
    images: usize,
    links: usize,
}

impl ParsedContent {
    fn new() -> Self {
        Self {
            elements: Vec::new(),
            headings: Vec::new(),
            paragraphs: 0,
            code_blocks: 0,
            tables: 0,
            images: 0,
            links: 0,
        }
    }
}

#[derive(Debug)]
enum ContentElement {
    Heading { level: u32, text: String },
    Paragraph { text: String },
    CodeBlock { language: Option<String>, code: String },
    Table { headers: Vec<String>, rows: Vec<Vec<String>>, alignments: Vec<Alignment> },
    ListStart { ordered: bool, start_number: Option<u64> },
    ListItem { text: String },
    ListEnd,
    Image { url: String, alt_text: String, title: Option<String> },
}

#[derive(Debug, Clone)]
struct HeadingInfo {
    level: u32,
    title: String,
    page_number: Option<u32>,
}

#[derive(Debug)]
pub struct PdfExportResult {
    pub file_path: std::path::PathBuf,
    pub file_size_bytes: u64,
    pub generation_time_ms: u64,
    pub page_count: usize,
    pub warnings: Vec<String>,
    pub content_stats: ContentStats,
}

#[derive(Debug)]
pub struct ContentStats {
    pub headings: usize,
    pub paragraphs: usize,
    pub code_blocks: usize,
    pub tables: usize,
    pub images: usize,
    pub links: usize,
}