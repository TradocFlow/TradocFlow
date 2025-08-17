pub mod project_manager;
pub mod project_service;
pub mod translation_service;
// New adapter for translation-memory crate
pub mod translation_memory_adapter;
// Old services - will be deprecated
// Translation memory services now use the external crate
pub mod translation_memory_integration_service;
#[cfg(test)]
pub mod translation_memory_integration_test;
pub mod chapter_service;
pub mod document_import_service;
pub mod simplified_document_import_service;
pub mod chunk_processor;
pub mod chunk_linking_service;
#[cfg(test)]
pub mod chunk_linking_service_tests;
pub mod editor_sync_service;
pub mod language_syntax_service;
#[cfg(test)]
pub mod split_pane_editor_integration_test;
pub mod markdown_service;
pub mod collaborative_editing_service;
#[cfg(test)]
pub mod collaborative_editing_service_tests;
pub mod user_management_service;
pub mod permission_service;
#[cfg(test)]
pub mod user_management_service_tests;
#[cfg(test)]
pub mod permission_service_tests;
pub mod export_service;
#[cfg(test)]
pub mod export_service_tests;
pub mod document_processing;
// PDF export services temporarily disabled due to API compatibility issues with genpdf 0.2.0
// pub mod pdf_export_service;

pub use project_manager::ProjectManager;
pub use project_service::ProjectService;
pub use translation_service::TranslationService;

// Re-export types from the new translation-memory crate
pub use tradocflow_translation_memory::{
    TradocFlowTranslationMemory,
    TranslationMemoryService as NewTranslationMemoryService,
    TerminologyService as NewTerminologyService,
    HighlightingService as NewHighlightingService,
    TranslationUnit as NewTranslationUnit,
    TranslationMatch,
    TranslationSuggestion as NewTranslationSuggestion,
    MatchType,
    MatchScore,
    Term as NewTerm,
    TerminologyImportResult as NewTerminologyImportResult,
    Language as NewLanguage,
    Domain,
    Quality,
    Metadata as NewMetadata,
    ComprehensiveSearchResult,
    TranslationMemoryError,
    Result as TMResult,
};

// New adapter services
pub use translation_memory_adapter::{
    TranslationMemoryAdapter, TerminologyServiceAdapter
};

// Legacy types for compatibility - these will need to be migrated
use serde::{Deserialize, Serialize};

/// Type of chunk linking operation (legacy compatibility)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkLinkType {
    /// Link chunks into a phrase group
    LinkedPhrase,
    /// Unlink previously linked chunks
    Unlinked,
    /// Merge chunks into a single unit
    Merged,
}

/// Source of a translation suggestion (legacy compatibility)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranslationSource {
    /// From translation memory
    Memory,
    /// From terminology database
    Terminology,
    /// From machine translation
    MachineTranslation,
    /// From user input
    Manual,
}

// Placeholder for legacy services - to be replaced with adapters
pub type TranslationMemoryService = TranslationMemoryAdapter;
pub type TerminologyService = TerminologyServiceAdapter;

// Legacy terminology types (temporary compatibility layer)
use std::collections::HashMap;
use uuid::Uuid;

/// Term highlight for UI display (legacy compatibility)
#[derive(Debug, Clone)]
pub struct TermHighlight {
    pub term_id: Uuid,
    pub term: String,
    pub start_position: usize,
    pub end_position: usize,
    pub highlight_type: HighlightType,
    pub definition: Option<String>,
    pub confidence: f32,
}

/// Highlight type for terminology (legacy compatibility)
#[derive(Debug, Clone)]
pub enum HighlightType {
    DoNotTranslate,
    Inconsistent,
    Suggestion,
    Validated,
}

/// Terminology suggestion (legacy compatibility)
#[derive(Debug, Clone)]
pub struct TerminologySuggestion {
    pub original_text: String,
    pub suggested_term: String,
    pub definition: Option<String>,
    pub confidence: f32,
    pub position: usize,
    pub reason: String,
}

/// Consistency check result (legacy compatibility)
#[derive(Debug, Clone)]
pub struct ConsistencyCheckResult {
    pub term: String,
    pub inconsistencies: Vec<LanguageInconsistency>,
}

/// Language inconsistency (legacy compatibility)
#[derive(Debug, Clone)]
pub struct LanguageInconsistency {
    pub language: String,
    pub expected_term: String,
    pub found_terms: Vec<String>,
    pub positions: Vec<usize>,
}

/// Placeholder for terminology highlighting service
pub struct TerminologyHighlightingService {
    terminology_service: std::sync::Arc<TerminologyServiceAdapter>,
}

impl TerminologyHighlightingService {
    pub fn new(terminology_service: std::sync::Arc<TerminologyServiceAdapter>) -> Self {
        Self { terminology_service }
    }
    
    pub async fn highlight_terms_in_text(&self, _text: &str, _project_id: Uuid, _language: &str) -> crate::Result<Vec<TermHighlight>> {
        // Placeholder implementation - to be integrated with new crate
        Ok(vec![])
    }
    
    pub async fn generate_terminology_suggestions(&self, _text: &str, _project_id: Uuid, _language: &str) -> crate::Result<Vec<TerminologySuggestion>> {
        // Placeholder implementation - to be integrated with new crate
        Ok(vec![])
    }
    
    pub async fn check_consistency_across_languages(&self, _texts: HashMap<String, String>, _project_id: Uuid) -> crate::Result<Vec<ConsistencyCheckResult>> {
        // Placeholder implementation - to be integrated with new crate
        Ok(vec![])
    }
    
    pub async fn update_highlighting_for_text_change(&self, _text: &str, _change_start: usize, _change_end: usize, _project_id: Uuid, _language: &str) -> crate::Result<Vec<TermHighlight>> {
        // Placeholder implementation - to be integrated with new crate
        Ok(vec![])
    }
    
    pub fn invalidate_cache(&self, _project_id: Uuid) {
        // Placeholder implementation - to be integrated with new crate
    }
}
pub use translation_memory_integration_service::{
    TranslationMemoryIntegrationService, IntegrationConfig, EditorSuggestion, 
    ConfidenceIndicator, IndicatorType, TextPosition as TmTextPosition, SearchFilters, TranslationStatistics
};
// Terminology services now use the external crate
// pub use terminology_service::TerminologyService;
// pub use terminology_highlighting_service::{
//     TerminologyHighlightingService, TermHighlight, HighlightType, ConsistencyCheckResult,
//     LanguageInconsistency, TerminologySuggestion
// };
pub use chapter_service::{ChapterService, CreateChapterRequest, UpdateChapterRequest, ChapterSummary, ChapterStatistics, ChapterSearchResult, SearchMatchType};
pub use document_import_service::{
    DocumentImportService, ImportDocument, ImportConfig, ImportResult, LanguageDocumentMap
};
pub use simplified_document_import_service::{
    SimplifiedDocumentImportService, ImportConfig as SimplifiedImportConfig, 
    DocumentImportResult, MultiDocumentImportResult, ImportError, Chapter, 
    ImportProgress, ImportStatistics, FileValidationResult,
    ChapterOrganizationConfig, ChapterSortingStrategy, ChapterTitleStrategy,
    TocConfig, ChapterValidationResult, ChapterStatistics as ImportChapterStatistics
};
pub use chunk_processor::{ChunkProcessor, ChunkingConfig, ChunkingStrategy, ProcessedChunk, ChunkingStats};
pub use chunk_linking_service::{
    ChunkLinkingService, LinkedPhraseGroup, ChunkSelection, SelectionMode, 
    LinkingResult, MergeOptions, MergeStrategy, PhraseStatistics
};
pub use editor_sync_service::{
    EditorSyncService, SyncEvent as EditorSyncEvent, SyncEventType as EditorSyncEventType, 
    SplitPaneConfig, SplitOrientation, LanguagePaneState
};
pub use language_syntax_service::{
    LanguageSyntaxService, LanguageSyntaxConfig, TextDirection, MarkdownExtension, 
    SpecialCharacter as SyntaxSpecialCharacter, SyntaxTheme
};
pub use collaborative_editing_service::{
    CollaborativeEditingService, UserSession, DocumentChange as CollabDocumentChange, ChangeType as CollabChangeType, 
    TranslationSuggestion, SuggestionStatus, Comment, CommentType, CommentContext,
    CollaborationEvent, UserPresenceUpdate, ConflictNotification, ConflictType,
    SuggestionVote, VoteType, CommentReply, SelectionRange
};
pub use user_management_service::{
    UserManagementService, User, UserProfile, UserPreferences, CreateUserRequest,
    UpdateUserRequest, TeamInvitation, InvitationStatus, InviteTeamMemberRequest,
    UserManagementError
};
pub use permission_service::{
    PermissionService, PermissionContext, PermissionGrant, GrantPermissionRequest,
    PermissionError
};
pub use markdown_service::{
    MarkdownService, MarkdownElement, Position, RenderedMarkdown, MarkdownMetadata
};
pub use export_service::{
    ExportService, ExportRequest, ExportConfiguration, ExportLayout, ExportJob,
    ExportStatus, ExportProgress, ExportedFile, ExportHistory
};
pub use document_processing::{
    DocumentProcessingService, ThreadSafeDocumentProcessor, DocumentProcessingConfig,
    ProcessedDocument, BatchProcessResult, BatchImportError, ProcessingStatistics,
    ImportProgressInfo, ImportStage, ProgressCallback
};

// New markdown processing services
pub mod markdown_text_processor;
pub mod markdown_processor;
pub mod document_state_manager;
pub mod markdown_integration_example;

pub use markdown_text_processor::{
    MarkdownTextProcessor, TextPosition, TextSelection, Cursor, TextOperation,
    MarkdownFormat, FindReplaceOptions, SearchScope, FindMatch, TextProcessorError
};
pub use markdown_processor::{
    MarkdownProcessor, MarkdownNode, TextRange, ValidationError, ValidationErrorType,
    Severity, MarkdownProcessingConfig, FormatDetection, ProcessingStatistics as MarkdownProcessingStatistics,
    MarkdownProcessorError, LinkValidator, CustomParser
};
pub use document_state_manager::{
    DocumentStateManager, DocumentChange, ChangeType, DocumentVersion, AutoSaveConfig,
    DocumentState, LineEnding, ConflictDetection, Conflict, ConflictType as DocConflictType, ConflictSeverity,
    ConflictResolutionStrategy, MemoryUsageInfo, DocumentStateError
};
pub use markdown_integration_example::{
    MarkdownEditorBackend, SlintIntegration, DocumentStats, AdvancedMarkdownProcessor,
    ProcessedMarkdownResult, PerformanceMonitor
};
pub mod multi_language_manual_import;
pub use multi_language_manual_import::{
    MultiLanguageManualImportService, SupportedLanguage, FolderScanResult,
    LanguageConflict, MultiLanguageImportConfig, MultiLanguageImportResult
};

// Focus management service
pub mod focus_management_service;
pub use focus_management_service::{
    FocusManagementService, FocusManagementBridge, EditorFocusState, 
    FocusEvent, FocusUpdateResult
};

// Sentence alignment services
pub mod sentence_alignment_service;
pub mod text_structure_analyzer;
pub mod alignment_cache_service;
pub mod multi_pane_alignment_service;
pub mod alignment_api_service;

#[cfg(test)]
pub mod alignment_integration_tests;

pub use sentence_alignment_service::{
    SentenceAlignmentService, LanguageProfile, SentenceBoundary, BoundaryType,
    SentenceAlignment, AlignmentMethod, ValidationStatus, AlignmentQualityIndicator,
    ProblemArea, AlignmentIssue, AlignmentConfig, AlignmentStatistics,
    AlignmentMLModel, AlignmentCorrection
};

pub use text_structure_analyzer::{
    TextStructureAnalyzer, StructureAnalysisConfig, StructureAnalysisResult,
    TextStructure, TextStructureType, ListType, StructureHierarchy,
    StructureStatistics, LanguageSpecificFeatures, WritingDirection,
    SpecialCharacter, CharacterContext, FormattingPattern, PatternType,
    PatternOccurrence
};

pub use alignment_cache_service::{
    AlignmentCacheService, AlignmentCacheConfig, AlignmentCacheEntry,
    CacheStatistics, PerformanceMetrics as CachePerformanceMetrics,
    EvictionStrategy, CacheMaintenanceTask, MaintenanceTaskType
};

pub use multi_pane_alignment_service::{
    MultiPaneAlignmentService, MultiPaneAlignmentConfig, TextPane,
    SyncEvent, SyncEventType, SyncState, QualityMonitoringResult,
    QualityIssue, QualityIssueType, QualityIssueSeverity,
    QualityRecommendation, RecommendationType, ImplementationEffort,
    PerformanceMetrics as AlignmentPerformanceMetrics
};

pub use alignment_api_service::{
    AlignmentApiService, AlignmentApiConfig, AddPaneRequest, AddPaneResponse,
    UpdatePaneRequest, UpdatePaneResponse, SyncCursorRequest, SyncCursorResponse,
    UserCorrectionRequest, UserCorrectionResponse, SystemStatusResponse,
    PaneInfo, SyncStateInfo, QualityMonitoringInfo, PerformanceMetricsInfo,
    SystemHealth, HealthStatus, AlignmentUpdate, AlignmentUpdateType,
    AlignmentUpdateData
};

// PDF export services temporarily disabled due to API compatibility issues with genpdf 0.2.0
// pub use pdf_export_service::{
//     PdfExportService, PdfExportConfig, PdfExportError, PdfExportErrorKind, 
//     ProgressCallback as PdfProgressCallback
// };