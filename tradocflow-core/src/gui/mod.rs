pub mod app;
pub mod client;
pub mod state;
// pub mod terminology_bridge; // Temporarily disabled
// pub mod translation_memory_bridge; // Temporarily disabled
//pub mod collaboration_bridge;
pub mod user_management_bridge;
pub mod markdown_editor_bridge;
pub mod export_bridge;
pub mod focus_management_bridge;
pub mod main_window_focus_bridge;
pub mod alignment_confidence_bridge;
pub mod enhanced_formatting_functions;
pub mod enhanced_markdown_bridge;

pub use app::App;
pub use state::AppState;
// pub use terminology_bridge::TerminologyBridge; // Temporarily disabled
// pub use translation_memory_bridge::{TranslationMemoryBridge, SlintTranslationSuggestion, SlintTranslationMatch}; // Temporarily disabled
//pub use collaboration_bridge::{CollaborationBridge, SlintUserPresence, SlintSuggestion, SlintComment};
pub use user_management_bridge::UserManagementBridge;
pub use markdown_editor_bridge::MarkdownEditorBridge;
pub use export_bridge::ExportBridge;
pub use focus_management_bridge::FocusManagementUIBridge;
pub use alignment_confidence_bridge::AlignmentConfidenceBridge;
pub use enhanced_formatting_functions::{EnhancedFormattingEngine, TextSelection, FormattingResult};
pub use enhanced_markdown_bridge::EnhancedMarkdownBridge;