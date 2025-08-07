//! Data models for translation memory system
//! 
//! Extracted and refactored from the original TradocFlow core translation models

pub mod translation_unit;
pub mod terminology;
pub mod chunk;
pub mod common;

// Re-export key types from translation_unit
pub use translation_unit::{
    TranslationUnit, 
    TranslationUnitBuilder, 
    TranslationMetadata, 
    TranslationMatch, 
    TranslationSuggestion,
    MatchType,
    MatchScore
};

// Re-export key types from terminology
pub use terminology::{
    Term, 
    Term as Terminology, // Alias for compatibility
    TerminologyCsvRecord, 
    TerminologyImportResult, 
    TerminologyImportError,
    ConflictResolution,
    TerminologyValidationConfig
};

// Re-export key types from chunk
pub use chunk::{
    ChunkMetadata, 
    ChunkMetadata as Chunk, // Alias for compatibility
    ChunkBuilder, 
    ChunkType
};

// Re-export key types from common
pub use common::{
    LanguagePair, 
    ValidationError, 
    TranslationStatus,
    Language, 
    Domain, 
    Quality, 
    Metadata
};