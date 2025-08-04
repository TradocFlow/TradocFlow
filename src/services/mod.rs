pub mod project_manager;
pub mod project_service;
pub mod translation_service;
pub mod translation_memory_service;
pub mod terminology_service;
pub mod terminology_highlighting_service;
pub mod chapter_service;
pub mod document_import_service;
pub mod chunk_processor;
pub mod chunk_linking_service;
pub mod editor_sync_service;

pub use project_manager::ProjectManager;
pub use project_service::ProjectService;
pub use translation_service::TranslationService;
pub use translation_memory_service::TranslationMemoryService;
pub use terminology_service::TerminologyService;
pub use terminology_highlighting_service::{
    TerminologyHighlightingService, TermHighlight, HighlightType, ConsistencyCheckResult,
    LanguageInconsistency, TerminologySuggestion
};
pub use chapter_service::{ChapterService, CreateChapterRequest, UpdateChapterRequest, ChapterSummary, ChapterStatistics, ChapterSearchResult, SearchMatchType};
pub use document_import_service::{
    DocumentImportService, ImportDocument, ImportConfig, ImportResult, LanguageDocumentMap
};
pub use chunk_processor::{ChunkProcessor, ChunkingConfig, ChunkingStrategy, ProcessedChunk, ChunkingStats};
pub use chunk_linking_service::{
    ChunkLinkingService, LinkedPhraseGroup, ChunkSelection, SelectionMode, 
    LinkingResult, MergeOptions, MergeStrategy, PhraseStatistics
};
pub use editor_sync_service::{
    EditorSyncService, SyncEvent, SyncEventType, SplitPaneConfig, SplitOrientation, LanguagePaneState
};