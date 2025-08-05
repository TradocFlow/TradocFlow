pub mod project_manager;
pub mod project_service;
pub mod translation_service;
pub mod translation_memory_service;
pub mod translation_memory_integration_service;
pub mod translation_memory_integration_test;
pub mod terminology_service;
pub mod terminology_highlighting_service;
pub mod chapter_service;
pub mod document_import_service;
pub mod chunk_processor;
pub mod chunk_linking_service;
pub mod editor_sync_service;
pub mod language_syntax_service;
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

pub use project_manager::ProjectManager;
pub use project_service::ProjectService;
pub use translation_service::TranslationService;
pub use translation_memory_service::TranslationMemoryService;
pub use translation_memory_integration_service::{
    TranslationMemoryIntegrationService, IntegrationConfig, EditorSuggestion, 
    ConfidenceIndicator, IndicatorType, TextPosition, SearchFilters, TranslationStatistics
};
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
pub use language_syntax_service::{
    LanguageSyntaxService, LanguageSyntaxConfig, TextDirection, MarkdownExtension, 
    SpecialCharacter, SyntaxTheme
};
pub use collaborative_editing_service::{
    CollaborativeEditingService, UserSession, DocumentChange, ChangeType, 
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