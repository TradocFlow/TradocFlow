//! Translation unit model and related types

// ValidationError imported but not used since we use our own error system
use crate::error::{Result, TranslationMemoryError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A translation unit represents a source-target text pair with associated metadata
/// This model is extracted from the original TradocFlow core translation models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslationUnit {
    /// Unique identifier
    pub id: Uuid,
    
    /// Project identifier
    pub project_id: Uuid,
    
    /// Chapter identifier
    pub chapter_id: Uuid,
    
    /// Chunk identifier
    pub chunk_id: Uuid,
    
    /// Source language
    pub source_language: crate::models::Language,
    
    /// Source language text
    pub source_text: String,
    
    /// Target language
    pub target_language: crate::models::Language,
    
    /// Target language text
    pub target_text: String,
    
    /// Confidence score (0.0-1.0)
    pub confidence_score: f32,
    
    /// Optional context information
    pub context: Option<String>,
    
    /// Translation metadata
    pub metadata: TranslationMetadata,
    
    /// When this unit was created
    pub created_at: DateTime<Utc>,
    
    /// When this unit was last modified
    pub updated_at: DateTime<Utc>,
}

/// Metadata for translation units
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TranslationMetadata {
    /// ID of the translator who created this unit
    pub translator_id: Option<String>,
    
    /// ID of the reviewer who reviewed this unit
    pub reviewer_id: Option<String>,
    
    /// Quality score assigned by reviewer
    pub quality_score: Option<f32>,
    
    /// Notes from translators and reviewers
    pub notes: Vec<String>,
    
    /// Tags for categorization and search
    pub tags: Vec<String>,
}

/// Translation match result with score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationMatch {
    /// The matching translation unit
    pub unit: TranslationUnit,
    
    /// Match score (0.0-1.0)
    pub score: f32,
    
    /// Type of match
    pub match_type: MatchType,
}

/// Types of translation matches
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchType {
    /// Exact match (100%)
    Exact,
    
    /// High confidence match (90-99%)
    High,
    
    /// Good match (70-89%)
    Good,
    
    /// Fair match (50-69%)
    Fair,
    
    /// Poor match (below 50%)
    Poor,
}

/// Translation suggestion with multiple options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationSuggestion {
    /// Source text being translated
    pub source_text: String,
    
    /// Source language
    pub source_language: String,
    
    /// Target language
    pub target_language: String,
    
    /// List of possible translations with scores
    pub suggestions: Vec<TranslationMatch>,
    
    /// Context for the translation
    pub context: Option<String>,
}

impl TranslationUnit {
    /// Create a new translation unit with validation
    pub fn new(
        project_id: Uuid,
        chapter_id: Uuid,
        chunk_id: Uuid,
        source_language: crate::models::Language,
        source_text: String,
        target_language: crate::models::Language,
        target_text: String,
        confidence_score: f32,
        context: Option<String>,
    ) -> Result<Self> {
        // Validate languages (using Language enum, so always valid)

        // Validate text content
        if source_text.trim().is_empty() {
            return Err(TranslationMemoryError::DataValidation(
                "Source text cannot be empty".to_string()
            ));
        }

        if target_text.trim().is_empty() {
            return Err(TranslationMemoryError::DataValidation(
                "Target text cannot be empty".to_string()
            ));
        }

        // Validate confidence score
        if !(0.0..=1.0).contains(&confidence_score) {
            return Err(TranslationMemoryError::DataValidation(
                "Confidence score must be between 0.0 and 1.0".to_string()
            ));
        }

        // Ensure source and target languages are different
        if source_language == target_language {
            return Err(TranslationMemoryError::DataValidation(
                "Source and target languages must be different".to_string()
            ));
        }

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            project_id,
            chapter_id,
            chunk_id,
            source_language,
            source_text,
            target_language,
            target_text,
            confidence_score,
            context,
            metadata: TranslationMetadata::default(),
            created_at: now,
            updated_at: now,
        })
    }
    
    /// Update the translation text and confidence score
    pub fn update_translation(&mut self, target_text: String, confidence_score: f32) -> Result<()> {
        if target_text.trim().is_empty() {
            return Err(TranslationMemoryError::DataValidation(
                "Target text cannot be empty".to_string()
            ));
        }

        if !(0.0..=1.0).contains(&confidence_score) {
            return Err(TranslationMemoryError::DataValidation(
                "Confidence score must be between 0.0 and 1.0".to_string()
            ));
        }

        self.target_text = target_text;
        self.confidence_score = confidence_score;
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// Calculate fuzzy match score against a query string
    pub fn fuzzy_match_score(&self, query: &str) -> MatchScore {
        MatchScore::calculate(&self.source_text, query)
    }
    
    /// Get match type based on confidence score
    pub fn match_type(&self) -> MatchType {
        match self.confidence_score {
            score if score >= 1.0 => MatchType::Exact,
            score if score >= 0.9 => MatchType::High,
            score if score >= 0.7 => MatchType::Good,
            score if score >= 0.5 => MatchType::Fair,
            _ => MatchType::Poor,
        }
    }
    
    /// Check if this unit matches the given language pair
    pub fn matches_language_pair(&self, source: &crate::models::Language, target: &crate::models::Language) -> bool {
        &self.source_language == source && &self.target_language == target
    }
    
    /// Check if this unit matches the given language codes (for compatibility)
    pub fn matches_language_codes(&self, source: &str, target: &str) -> bool {
        self.source_language.code() == source && self.target_language.code() == target
    }
    
    /// Add a tag if it doesn't already exist
    pub fn add_tag(&mut self, tag: String) {
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
        }
    }
    
    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) {
        self.metadata.tags.retain(|t| t != tag);
    }
    
    /// Check if unit has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.metadata.tags.contains(&tag.to_string())
    }
    
    /// Add a note
    pub fn add_note(&mut self, note: String) {
        if !note.trim().is_empty() {
            self.metadata.notes.push(note);
        }
    }
    
    /// Get the age of this translation unit
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }
    
    /// Check if this unit is stale (older than given duration)
    pub fn is_stale(&self, max_age: chrono::Duration) -> bool {
        self.age() > max_age
    }
    
    /// Validate the translation unit for consistency
    pub fn validate(&self) -> Result<()> {
        // Language validation (using enum, so always valid)

        if self.source_text.trim().is_empty() {
            return Err(TranslationMemoryError::DataValidation(
                "Source text cannot be empty".to_string()
            ));
        }

        if self.target_text.trim().is_empty() {
            return Err(TranslationMemoryError::DataValidation(
                "Target text cannot be empty".to_string()
            ));
        }

        if !(0.0..=1.0).contains(&self.confidence_score) {
            return Err(TranslationMemoryError::DataValidation(
                "Confidence score must be between 0.0 and 1.0".to_string()
            ));
        }

        if self.source_language == self.target_language {
            return Err(TranslationMemoryError::DataValidation(
                "Source and target languages must be different".to_string()
            ));
        }
        
        if self.created_at > self.updated_at {
            return Err(TranslationMemoryError::DataValidation(
                "Created date cannot be after updated date".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Builder pattern for creating translation units
#[derive(Debug, Default)]
pub struct TranslationUnitBuilder {
    project_id: Option<Uuid>,
    chapter_id: Option<Uuid>,
    chunk_id: Option<Uuid>,
    source_text: Option<String>,
    target_text: Option<String>,
    source_language: Option<crate::models::Language>,
    target_language: Option<crate::models::Language>,
    confidence_score: Option<f32>,
    context: Option<String>,
    metadata: Option<TranslationMetadata>,
}

impl TranslationUnitBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set project ID
    pub fn project_id(mut self, id: Uuid) -> Self {
        self.project_id = Some(id);
        self
    }
    
    /// Set chapter ID
    pub fn chapter_id(mut self, id: Uuid) -> Self {
        self.chapter_id = Some(id);
        self
    }
    
    /// Set chunk ID
    pub fn chunk_id(mut self, id: Uuid) -> Self {
        self.chunk_id = Some(id);
        self
    }
    
    /// Set source text
    pub fn source_text<S: Into<String>>(mut self, text: S) -> Self {
        self.source_text = Some(text.into());
        self
    }
    
    /// Set target text
    pub fn target_text<S: Into<String>>(mut self, text: S) -> Self {
        self.target_text = Some(text.into());
        self
    }
    
    /// Set source language from code
    pub fn source_language<S: AsRef<str>>(mut self, lang_code: S) -> Self {
        if let Some(lang) = crate::models::Language::from_code(lang_code.as_ref()) {
            self.source_language = Some(lang);
        }
        self
    }
    
    /// Set source language directly
    pub fn source_language_enum(mut self, lang: crate::models::Language) -> Self {
        self.source_language = Some(lang);
        self
    }
    
    /// Set target language from code
    pub fn target_language<S: AsRef<str>>(mut self, lang_code: S) -> Self {
        if let Some(lang) = crate::models::Language::from_code(lang_code.as_ref()) {
            self.target_language = Some(lang);
        }
        self
    }
    
    /// Set target language directly
    pub fn target_language_enum(mut self, lang: crate::models::Language) -> Self {
        self.target_language = Some(lang);
        self
    }
    
    /// Set confidence score
    pub fn confidence_score(mut self, score: f32) -> Self {
        self.confidence_score = Some(score);
        self
    }
    
    /// Set context
    pub fn context<S: Into<String>>(mut self, context: S) -> Self {
        self.context = Some(context.into());
        self
    }
    
    /// Set metadata
    pub fn metadata(mut self, metadata: TranslationMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
    
    /// Build the translation unit
    pub fn build(self) -> Result<TranslationUnit> {
        let project_id = self.project_id
            .ok_or_else(|| TranslationMemoryError::DataValidation("Project ID is required".to_string()))?;
        let chapter_id = self.chapter_id
            .ok_or_else(|| TranslationMemoryError::DataValidation("Chapter ID is required".to_string()))?;
        let chunk_id = self.chunk_id
            .ok_or_else(|| TranslationMemoryError::DataValidation("Chunk ID is required".to_string()))?;
        let source_text = self.source_text
            .ok_or_else(|| TranslationMemoryError::DataValidation("Source text is required".to_string()))?;
        let target_text = self.target_text
            .ok_or_else(|| TranslationMemoryError::DataValidation("Target text is required".to_string()))?;
        let source_language = self.source_language
            .ok_or_else(|| TranslationMemoryError::DataValidation("Source language is required".to_string()))?;
        let target_language = self.target_language
            .ok_or_else(|| TranslationMemoryError::DataValidation("Target language is required".to_string()))?;
        
        TranslationUnit::new(
            project_id,
            chapter_id,
            chunk_id,
            source_language,
            source_text,
            target_language,
            target_text,
            self.confidence_score.unwrap_or(0.0),
            self.context,
        )
    }
}

/// Match score for fuzzy matching
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct MatchScore(f64);

impl MatchScore {
    /// Create a new match score (0.0-1.0)
    pub fn new(score: f64) -> Result<Self> {
        if !(0.0..=1.0).contains(&score) {
            Err(TranslationMemoryError::InvalidMatchScore(score))
        } else {
            Ok(Self(score))
        }
    }
    
    /// Get the score as f64
    pub fn score(&self) -> f64 {
        self.0
    }
    
    /// Get the score as percentage
    pub fn percentage(&self) -> u8 {
        (self.0 * 100.0).round() as u8
    }
    
    /// Check if this is an exact match
    pub fn is_exact(&self) -> bool {
        self.0 >= 0.999
    }
    
    /// Check if this is a high-quality match (>= 90%)
    pub fn is_high_quality(&self) -> bool {
        self.0 >= 0.9
    }
    
    /// Check if this is a good match (>= 70%)
    pub fn is_good(&self) -> bool {
        self.0 >= 0.7
    }
    
    /// Calculate fuzzy match score between two strings
    pub fn calculate(text1: &str, text2: &str) -> Self {
        if text1 == text2 {
            return Self(1.0);
        }
        
        if text1.is_empty() || text2.is_empty() {
            return Self(0.0);
        }
        
        // Simple Levenshtein-based similarity
        let distance = levenshtein_distance(text1, text2);
        let max_len = text1.len().max(text2.len());
        let similarity = 1.0 - (distance as f64 / max_len as f64);
        
        Self(similarity.max(0.0))
    }
    
    /// Common match score thresholds
    pub const EXACT: MatchScore = MatchScore(1.0);
    pub const HIGH: MatchScore = MatchScore(0.9);
    pub const GOOD: MatchScore = MatchScore(0.7);
    pub const FAIR: MatchScore = MatchScore(0.5);
    pub const POOR: MatchScore = MatchScore(0.3);
}

impl std::fmt::Display for MatchScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.percentage())
    }
}

/// Simple Levenshtein distance calculation
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let chars1: Vec<char> = s1.chars().collect();
    let chars2: Vec<char> = s2.chars().collect();
    let len1 = chars1.len();
    let len2 = chars2.len();
    
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }
    
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
    
    // Initialize first row and column
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }
    
    // Calculate distances
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    
    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_translation_unit_creation() {
        let unit = TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            crate::models::Language::English,
            "Hello world".to_string(),
            crate::models::Language::Spanish,
            "Hola mundo".to_string(),
            0.95,
            Some("Greeting context".to_string()),
        );
        
        assert!(unit.is_ok());
        let unit = unit.unwrap();
        assert_eq!(unit.source_text, "Hello world");
        assert_eq!(unit.target_text, "Hola mundo");
        assert_eq!(unit.source_language, crate::models::Language::English);
        assert_eq!(unit.target_language, crate::models::Language::Spanish);
        assert_eq!(unit.confidence_score, 0.95);
    }
    
    #[test]
    fn test_translation_unit_builder() {
        let unit = TranslationUnitBuilder::new()
            .project_id(Uuid::new_v4())
            .chapter_id(Uuid::new_v4())
            .chunk_id(Uuid::new_v4())
            .source_text("Hello")
            .target_text("Hola")
            .source_language("en")
            .target_language("es")
            .confidence_score(0.9)
            .build();
        
        assert!(unit.is_ok());
        let unit = unit.unwrap();
        assert_eq!(unit.source_text, "Hello");
        assert_eq!(unit.target_text, "Hola");
    }
    
    #[test]
    fn test_match_score_calculation() {
        let score = MatchScore::calculate("hello", "hello");
        assert!(score.is_exact());
        
        let score = MatchScore::calculate("hello", "helo");
        assert!(score.is_good());
        
        let score = MatchScore::calculate("hello", "world");
        assert!(!score.is_good());
    }
    
    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("hello", "helo"), 1);
        assert_eq!(levenshtein_distance("hello", "world"), 4);
    }
    
    #[test]
    fn test_translation_unit_validation() {
        let unit = TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            crate::models::Language::English,
            "".to_string(), // Empty source text
            crate::models::Language::Spanish,
            "Hola".to_string(),
            0.95,
            None,
        );
        
        assert!(unit.is_err());
        
        let unit = TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            crate::models::Language::English,
            "Hello".to_string(),
            crate::models::Language::English, // Same language
            "Hello".to_string(),
            0.95,
            None,
        );
        
        assert!(unit.is_err());
    }
    
    #[test]
    fn test_translation_unit_tags_and_notes() {
        let mut unit = TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            crate::models::Language::English,
            "Hello".to_string(),
            crate::models::Language::Spanish,
            "Hola".to_string(),
            0.95,
            None,
        ).unwrap();
        
        unit.add_tag("greeting".to_string());
        assert!(unit.has_tag("greeting"));
        
        unit.add_note("Informal greeting".to_string());
        assert_eq!(unit.metadata.notes.len(), 1);
        
        unit.remove_tag("greeting");
        assert!(!unit.has_tag("greeting"));
    }
}