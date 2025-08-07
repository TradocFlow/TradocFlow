# Design Document

## Overview

This design document outlines the solution for fixing TradocFlow Core compilation issues and implementing a functional markdown editor with dual-mode editing and Word document import capabilities. The design focuses on resolving dependency conflicts, fixing import issues, and creating a clean architecture for the markdown editor.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    TradocFlow Core                          │
├─────────────────────────────────────────────────────────────┤
│  GUI Layer (Slint)                                         │
│  ┌─────────────────┐  ┌─────────────────┐                 │
│  │ Markdown Editor │  │ Import Dialog   │                 │
│  │ Component       │  │ Component       │                 │
│  └─────────────────┘  └─────────────────┘                 │
├─────────────────────────────────────────────────────────────┤
│  Bridge Layer                                               │
│  ┌─────────────────┐  ┌─────────────────┐                 │
│  │ Editor Bridge   │  │ Import Bridge   │                 │
│  └─────────────────┘  └─────────────────┘                 │
├─────────────────────────────────────────────────────────────┤
│  Service Layer                                              │
│  ┌─────────────────┐  ┌─────────────────┐                 │
│  │ Markdown        │  │ Document Import │                 │
│  │ Service         │  │ Service         │                 │
│  └─────────────────┘  └─────────────────┘                 │
├─────────────────────────────────────────────────────────────┤
│  Storage Layer                                              │
│  ┌─────────────────┐  ┌─────────────────┐                 │
│  │ File System     │  │ SQLite Database │                 │
│  └─────────────────┘  └─────────────────┘                 │
└─────────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### 1. Compilation Fix Strategy

#### Problem Analysis
The main compilation issues are:
1. Missing `translation_memory_service` module
2. Incorrect `TranslationUnit` imports and type mismatches
3. Missing Slint UI method implementations
4. Enum variant naming inconsistencies
5. Type conversion issues between core and translation-memory crate

#### Solution Approach
1. **Disable Translation Memory Integration Temporarily**: Comment out or stub translation memory dependencies to get core functionality working
2. **Fix Import Paths**: Correct all import statements to use existing modules
3. **Implement Missing Slint Methods**: Add required UI method stubs
4. **Fix Type Mismatches**: Align types between different modules
5. **Create Minimal Working Version**: Focus on core markdown editing without advanced features

### 2. Markdown Editor Component

#### Core Structure
```rust
pub struct MarkdownEditor {
    content: String,
    mode: EditorMode,
    language: String,
    auto_save: bool,
    undo_stack: Vec<String>,
    redo_stack: Vec<String>,
}

pub enum EditorMode {
    Markdown,
    Preview,
    Split,
}
```

#### Key Features
- **Dual Mode Editing**: Switch between markdown source and WYSIWYG preview
- **Real-time Preview**: Live rendering of markdown to HTML
- **Syntax Highlighting**: Basic markdown syntax highlighting in source mode
- **Toolbar Integration**: Rich formatting toolbar with common operations
- **Keyboard Shortcuts**: Standard editing shortcuts (Ctrl+B, Ctrl+I, etc.)

### 3. Document Import Service

#### Import Pipeline
```
Word Document → Text Extraction → Markdown Conversion → Editor Integration
```

#### Supported Formats
- `.docx` files (using docx-rs crate)
- `.doc` files (basic text extraction)
- `.txt` files (direct conversion)
- `.md` files (direct loading)

#### Import Process
1. **File Selection**: Multi-file selection dialog
2. **Format Detection**: Automatic format detection based on extension
3. **Text Extraction**: Extract text content from documents
4. **Markdown Conversion**: Convert extracted text to markdown format
5. **Chapter Organization**: Organize multiple documents as chapters
6. **Editor Loading**: Load converted content into the editor

### 4. Service Layer Design

#### Markdown Service
```rust
pub struct MarkdownService {
    parser: ComrakOptions,
    renderer: HtmlRenderer,
}

impl MarkdownService {
    pub fn render_to_html(&self, markdown: &str) -> Result<String>;
    pub fn parse_to_elements(&self, markdown: &str) -> Result<Vec<MarkdownElement>>;
    pub fn apply_formatting(&self, text: &str, format: FormatType) -> Result<String>;
    pub fn validate_syntax(&self, markdown: &str) -> Result<Vec<ValidationError>>;
}
```

#### Document Import Service (Simplified)
```rust
pub struct DocumentImportService {
    supported_formats: Vec<String>,
}

impl DocumentImportService {
    pub async fn import_documents(&self, files: Vec<PathBuf>) -> Result<Vec<Chapter>>;
    pub async fn convert_docx(&self, path: &Path) -> Result<String>;
    pub async fn convert_doc(&self, path: &Path) -> Result<String>;
    pub async fn convert_text(&self, path: &Path) -> Result<String>;
}
```

### 5. UI Bridge Layer

#### Editor Bridge
```rust
pub struct MarkdownEditorBridge {
    service: MarkdownService,
    current_content: Arc<Mutex<String>>,
    current_mode: Arc<Mutex<EditorMode>>,
}

impl MarkdownEditorBridge {
    pub fn update_content(&self, content: String) -> Result<()>;
    pub fn get_rendered_html(&self) -> Result<String>;
    pub fn apply_formatting(&self, format: FormatType) -> Result<()>;
    pub fn toggle_mode(&self) -> Result<EditorMode>;
}
```

#### Import Bridge
```rust
pub struct ImportBridge {
    import_service: DocumentImportService,
    progress_callback: Option<Box<dyn Fn(ImportProgress)>>,
}

impl ImportBridge {
    pub async fn import_files(&self, files: Vec<PathBuf>) -> Result<ImportResult>;
    pub fn set_progress_callback(&mut self, callback: Box<dyn Fn(ImportProgress)>);
}
```

## Data Models

### Core Data Structures

#### Document Model
```rust
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub content: HashMap<String, String>, // language -> content
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub language: String,
}
```

#### Chapter Model
```rust
pub struct Chapter {
    pub id: Uuid,
    pub document_id: Uuid,
    pub chapter_number: u32,
    pub title: String,
    pub content: String,
    pub language: String,
}
```

#### Import Models
```rust
pub struct ImportRequest {
    pub files: Vec<PathBuf>,
    pub target_language: String,
    pub chapter_mode: bool,
}

pub struct ImportResult {
    pub success: bool,
    pub documents: Vec<Document>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct ImportProgress {
    pub current_file: String,
    pub progress_percent: u8,
    pub message: String,
}
```

## Error Handling

### Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum MarkdownEditorError {
    #[error("Markdown parsing error: {0}")]
    ParseError(String),
    
    #[error("File operation error: {0}")]
    FileError(#[from] std::io::Error),
    
    #[error("Import error: {0}")]
    ImportError(String),
    
    #[error("Rendering error: {0}")]
    RenderError(String),
    
    #[error("UI error: {0}")]
    UiError(String),
}
```

### Error Handling Strategy
1. **Graceful Degradation**: Continue operation when non-critical errors occur
2. **User-Friendly Messages**: Convert technical errors to user-understandable messages
3. **Recovery Options**: Provide options to retry or work around errors
4. **Logging**: Log detailed error information for debugging

## Testing Strategy

### Unit Tests
- **Markdown Service**: Test parsing, rendering, and formatting functions
- **Import Service**: Test document conversion for each supported format
- **Bridge Layer**: Test UI integration and state management
- **Error Handling**: Test error scenarios and recovery

### Integration Tests
- **End-to-End Import**: Test complete import workflow
- **Editor Functionality**: Test dual-mode editing and synchronization
- **File Operations**: Test save/load operations
- **UI Integration**: Test Slint component integration

### Test Data
- Sample Word documents with various formatting
- Markdown files with different syntax elements
- Edge cases (empty files, corrupted documents, large files)

## Implementation Phases

### Phase 1: Compilation Fixes (Priority: Critical)
1. Fix all import statements and missing modules
2. Stub out translation memory dependencies
3. Implement missing Slint UI methods
4. Fix type mismatches and enum variants
5. Ensure clean compilation

### Phase 2: Basic Markdown Editor (Priority: High)
1. Implement core MarkdownService
2. Create basic editor UI component
3. Add markdown-to-HTML rendering
4. Implement mode switching
5. Add basic toolbar functionality

### Phase 3: Document Import (Priority: High)
1. Implement DocumentImportService
2. Add DOCX parsing and conversion
3. Create import UI dialog
4. Add progress tracking
5. Integrate with editor

### Phase 4: Enhanced Features (Priority: Medium)
1. Add syntax highlighting
2. Implement inline editing in preview mode
3. Add advanced formatting options
4. Improve error handling and user feedback
5. Add keyboard shortcuts

### Phase 5: Polish and Optimization (Priority: Low)
1. Performance optimization
2. UI/UX improvements
3. Additional file format support
4. Advanced markdown features
5. Comprehensive testing

## Security Considerations

### File Handling Security
- **Path Validation**: Validate file paths to prevent directory traversal
- **File Size Limits**: Implement reasonable file size limits
- **Format Validation**: Validate file formats before processing
- **Sandboxing**: Process documents in isolated environment when possible

### Data Protection
- **Temporary Files**: Clean up temporary files after processing
- **Memory Management**: Prevent memory leaks with large documents
- **Error Information**: Avoid exposing sensitive information in error messages

## Performance Considerations

### Optimization Strategies
- **Lazy Loading**: Load and render content on demand
- **Incremental Updates**: Update only changed portions of preview
- **Caching**: Cache rendered HTML for unchanged content
- **Background Processing**: Process imports in background threads
- **Memory Management**: Efficient memory usage for large documents

### Performance Targets
- **Startup Time**: < 2 seconds for application startup
- **Import Time**: < 5 seconds per document for typical Word files
- **Rendering Time**: < 500ms for preview updates
- **Memory Usage**: < 100MB for typical document editing sessions