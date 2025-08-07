//! Services for translation memory operations

pub mod translation_memory;
pub mod terminology;
pub mod highlighting;

// Re-export key services
pub use translation_memory::TranslationMemoryService;
pub use terminology::TerminologyService;
pub use highlighting::HighlightingService;