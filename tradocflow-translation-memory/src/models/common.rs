//! Common models and types
//! 
//! Extracted from the original TradocFlow core translation models

use serde::{Deserialize, Serialize};

/// Language pair for translation operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct LanguagePair {
    /// Source language code
    pub source: String,
    
    /// Target language code
    pub target: String,
}

impl LanguagePair {
    /// Create a new language pair
    pub fn new(source: String, target: String) -> Self {
        Self { source, target }
    }
    
    /// Create from language codes
    pub fn from_codes(source: &str, target: &str) -> Self {
        Self {
            source: source.to_string(),
            target: target.to_string(),
        }
    }
    
    /// Get the reverse language pair
    pub fn reverse(&self) -> Self {
        Self {
            source: self.target.clone(),
            target: self.source.clone(),
        }
    }
    
    /// Check if this is a valid language pair (different languages)
    pub fn is_valid(&self) -> bool {
        !self.source.is_empty() && !self.target.is_empty() && self.source != self.target
    }
}

impl std::fmt::Display for LanguagePair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} → {}", self.source, self.target)
    }
}

/// Validation errors for translation models
/// Extracted from the original TradocFlow core translation models
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid project name: {0}")]
    InvalidProjectName(String),
    
    #[error("Invalid language: {0}")]
    InvalidLanguage(String),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Duplicate member: {0}")]
    DuplicateMember(String),
    
    #[error("Member not found: {0}")]
    MemberNotFound(String),
    
    #[error("Invalid member ID: {0}")]
    InvalidMemberId(String),
    
    #[error("Invalid member name: {0}")]
    InvalidMemberName(String),
    
    #[error("Invalid email: {0}")]
    InvalidEmail(String),
    
    #[error("Invalid translation unit: {0}")]
    InvalidTranslationUnit(String),
    
    #[error("Invalid chunk: {0}")]
    InvalidChunk(String),
    
    #[error("Invalid term: {0}")]
    InvalidTerm(String),
}

/// Translation status tracking for workflow management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranslationStatus {
    /// Translation not started
    NotStarted,
    
    /// Translation in progress
    InProgress,
    
    /// Pending review
    PendingReview,
    
    /// Approved by reviewer
    Approved,
    
    /// Rejected by reviewer
    Rejected,
    
    /// Published and finalized
    Published,
}

impl TranslationStatus {
    /// Get all available translation statuses
    pub fn all() -> Vec<TranslationStatus> {
        vec![
            TranslationStatus::NotStarted,
            TranslationStatus::InProgress,
            TranslationStatus::PendingReview,
            TranslationStatus::Approved,
            TranslationStatus::Rejected,
            TranslationStatus::Published,
        ]
    }

    /// Get status description
    pub fn description(&self) -> &'static str {
        match self {
            TranslationStatus::NotStarted => "Translation not started",
            TranslationStatus::InProgress => "Translation in progress",
            TranslationStatus::PendingReview => "Pending review",
            TranslationStatus::Approved => "Approved by reviewer",
            TranslationStatus::Rejected => "Rejected by reviewer",
            TranslationStatus::Published => "Published and finalized",
        }
    }

    /// Check if status allows editing
    pub fn allows_editing(&self) -> bool {
        matches!(self, TranslationStatus::NotStarted | TranslationStatus::InProgress | TranslationStatus::Rejected)
    }

    /// Check if status requires review
    pub fn requires_review(&self) -> bool {
        matches!(self, TranslationStatus::PendingReview)
    }

    /// Get next possible statuses
    pub fn next_statuses(&self) -> Vec<TranslationStatus> {
        match self {
            TranslationStatus::NotStarted => vec![TranslationStatus::InProgress],
            TranslationStatus::InProgress => vec![TranslationStatus::PendingReview],
            TranslationStatus::PendingReview => vec![TranslationStatus::Approved, TranslationStatus::Rejected],
            TranslationStatus::Approved => vec![TranslationStatus::Published],
            TranslationStatus::Rejected => vec![TranslationStatus::InProgress],
            TranslationStatus::Published => vec![], // Final status
        }
    }
}

/// Common language codes used in translation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    English,
    Spanish,
    French,
    German,
    Italian,
    Portuguese,
    Russian,
    Chinese,
    Japanese,
    Korean,
    Arabic,
    Hindi,
    Dutch,
    Swedish,
    Norwegian,
    Danish,
    Finnish,
    Polish,
    Czech,
    Hungarian,
    Romanian,
    Bulgarian,
    Croatian,
    Serbian,
    Slovenian,
    Slovak,
    Estonian,
    Latvian,
    Lithuanian,
    Maltese,
    Irish,
    Welsh,
    Basque,
    Catalan,
    Galician,
    Turkish,
    Greek,
    Hebrew,
    Thai,
    Vietnamese,
    Indonesian,
    Malay,
    Filipino,
    Swahili,
    Afrikaans,
    Zulu,
    Xhosa,
    Yoruba,
    Hausa,
    Amharic,
    Custom(String),
}

impl Language {
    /// Get the ISO 639-1 language code
    pub fn code(&self) -> &str {
        match self {
            Language::English => "en",
            Language::Spanish => "es",
            Language::French => "fr",
            Language::German => "de",
            Language::Italian => "it",
            Language::Portuguese => "pt",
            Language::Russian => "ru",
            Language::Chinese => "zh",
            Language::Japanese => "ja",
            Language::Korean => "ko",
            Language::Arabic => "ar",
            Language::Hindi => "hi",
            Language::Dutch => "nl",
            Language::Swedish => "sv",
            Language::Norwegian => "no",
            Language::Danish => "da",
            Language::Finnish => "fi",
            Language::Polish => "pl",
            Language::Czech => "cs",
            Language::Hungarian => "hu",
            Language::Romanian => "ro",
            Language::Bulgarian => "bg",
            Language::Croatian => "hr",
            Language::Serbian => "sr",
            Language::Slovenian => "sl",
            Language::Slovak => "sk",
            Language::Estonian => "et",
            Language::Latvian => "lv",
            Language::Lithuanian => "lt",
            Language::Maltese => "mt",
            Language::Irish => "ga",
            Language::Welsh => "cy",
            Language::Basque => "eu",
            Language::Catalan => "ca",
            Language::Galician => "gl",
            Language::Turkish => "tr",
            Language::Greek => "el",
            Language::Hebrew => "he",
            Language::Thai => "th",
            Language::Vietnamese => "vi",
            Language::Indonesian => "id",
            Language::Malay => "ms",
            Language::Filipino => "fil",
            Language::Swahili => "sw",
            Language::Afrikaans => "af",
            Language::Zulu => "zu",
            Language::Xhosa => "xh",
            Language::Yoruba => "yo",
            Language::Hausa => "ha",
            Language::Amharic => "am",
            Language::Custom(code) => code,
        }
    }
    
    /// Get the display name
    pub fn name(&self) -> &str {
        match self {
            Language::English => "English",
            Language::Spanish => "Spanish",
            Language::French => "French",
            Language::German => "German",
            Language::Italian => "Italian",
            Language::Portuguese => "Portuguese",
            Language::Russian => "Russian",
            Language::Chinese => "Chinese",
            Language::Japanese => "Japanese",
            Language::Korean => "Korean",
            Language::Arabic => "Arabic",
            Language::Hindi => "Hindi",
            Language::Dutch => "Dutch",
            Language::Swedish => "Swedish",
            Language::Norwegian => "Norwegian",
            Language::Danish => "Danish",
            Language::Finnish => "Finnish",
            Language::Polish => "Polish",
            Language::Czech => "Czech",
            Language::Hungarian => "Hungarian",
            Language::Romanian => "Romanian",
            Language::Bulgarian => "Bulgarian",
            Language::Croatian => "Croatian",
            Language::Serbian => "Serbian",
            Language::Slovenian => "Slovenian",
            Language::Slovak => "Slovak",
            Language::Estonian => "Estonian",
            Language::Latvian => "Latvian",
            Language::Lithuanian => "Lithuanian",
            Language::Maltese => "Maltese",
            Language::Irish => "Irish",
            Language::Welsh => "Welsh",
            Language::Basque => "Basque",
            Language::Catalan => "Catalan",
            Language::Galician => "Galician",
            Language::Turkish => "Turkish",
            Language::Greek => "Greek",
            Language::Hebrew => "Hebrew",
            Language::Thai => "Thai",
            Language::Vietnamese => "Vietnamese",
            Language::Indonesian => "Indonesian",
            Language::Malay => "Malay",
            Language::Filipino => "Filipino",
            Language::Swahili => "Swahili",
            Language::Afrikaans => "Afrikaans",
            Language::Zulu => "Zulu",
            Language::Xhosa => "Xhosa",
            Language::Yoruba => "Yoruba",
            Language::Hausa => "Hausa",
            Language::Amharic => "Amharic",
            Language::Custom(name) => name,
        }
    }
    
    /// Parse from ISO 639-1 code
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "en" => Some(Language::English),
            "es" => Some(Language::Spanish),
            "fr" => Some(Language::French),
            "de" => Some(Language::German),
            "it" => Some(Language::Italian),
            "pt" => Some(Language::Portuguese),
            "ru" => Some(Language::Russian),
            "zh" => Some(Language::Chinese),
            "ja" => Some(Language::Japanese),
            "ko" => Some(Language::Korean),
            "ar" => Some(Language::Arabic),
            "hi" => Some(Language::Hindi),
            "nl" => Some(Language::Dutch),
            "sv" => Some(Language::Swedish),
            "no" => Some(Language::Norwegian),
            "da" => Some(Language::Danish),
            "fi" => Some(Language::Finnish),
            "pl" => Some(Language::Polish),
            "cs" => Some(Language::Czech),
            "hu" => Some(Language::Hungarian),
            "ro" => Some(Language::Romanian),
            "bg" => Some(Language::Bulgarian),
            "hr" => Some(Language::Croatian),
            "sr" => Some(Language::Serbian),
            "sl" => Some(Language::Slovenian),
            "sk" => Some(Language::Slovak),
            "et" => Some(Language::Estonian),
            "lv" => Some(Language::Latvian),
            "lt" => Some(Language::Lithuanian),
            "mt" => Some(Language::Maltese),
            "ga" => Some(Language::Irish),
            "cy" => Some(Language::Welsh),
            "eu" => Some(Language::Basque),
            "ca" => Some(Language::Catalan),
            "gl" => Some(Language::Galician),
            "tr" => Some(Language::Turkish),
            "el" => Some(Language::Greek),
            "he" => Some(Language::Hebrew),
            "th" => Some(Language::Thai),
            "vi" => Some(Language::Vietnamese),
            "id" => Some(Language::Indonesian),
            "ms" => Some(Language::Malay),
            "fil" => Some(Language::Filipino),
            "sw" => Some(Language::Swahili),
            "af" => Some(Language::Afrikaans),
            "zu" => Some(Language::Zulu),
            "xh" => Some(Language::Xhosa),
            "yo" => Some(Language::Yoruba),
            "ha" => Some(Language::Hausa),
            "am" => Some(Language::Amharic),
            _ => Some(Language::Custom(code.to_string())),
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Domain/subject areas for content categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    General,
    Technical,
    Medical,
    Legal,
    Financial,
    Marketing,
    Education,
    Science,
    Literature,
    News,
    Software,
    Gaming,
    Travel,
    Cooking,
    Sports,
    Music,
    Art,
    History,
    Politics,
    Religion,
    Business,
    Automotive,
    Fashion,
    Health,
    Environment,
}

impl Domain {
    /// Get domain description
    pub fn description(&self) -> &'static str {
        match self {
            Domain::General => "General content",
            Domain::Technical => "Technical documentation",
            Domain::Medical => "Medical and healthcare",
            Domain::Legal => "Legal documents",
            Domain::Financial => "Financial and banking",
            Domain::Marketing => "Marketing materials",
            Domain::Education => "Educational content",
            Domain::Science => "Scientific literature",
            Domain::Literature => "Literary works",
            Domain::News => "News and journalism",
            Domain::Software => "Software documentation",
            Domain::Gaming => "Gaming content",
            Domain::Travel => "Travel and tourism",
            Domain::Cooking => "Culinary content",
            Domain::Sports => "Sports content",
            Domain::Music => "Music and audio",
            Domain::Art => "Art and design",
            Domain::History => "Historical content",
            Domain::Politics => "Political content",
            Domain::Religion => "Religious content",
            Domain::Business => "Business content",
            Domain::Automotive => "Automotive content",
            Domain::Fashion => "Fashion and lifestyle",
            Domain::Health => "Health and wellness",
            Domain::Environment => "Environmental content",
        }
    }
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Quality levels for translations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Quality {
    Draft,
    Review,
    Approved,
    Published,
}

impl Quality {
    /// Get quality description
    pub fn description(&self) -> &'static str {
        match self {
            Quality::Draft => "Draft quality - needs review",
            Quality::Review => "Under review",
            Quality::Approved => "Approved for use",
            Quality::Published => "Published quality",
        }
    }
    
    /// Get quality score (0-100)
    pub fn score(&self) -> u8 {
        match self {
            Quality::Draft => 25,
            Quality::Review => 50,
            Quality::Approved => 75,
            Quality::Published => 100,
        }
    }
}

impl Default for Quality {
    fn default() -> Self {
        Quality::Draft
    }
}

impl std::fmt::Display for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Generic metadata container
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    /// Custom metadata fields
    pub fields: std::collections::HashMap<String, String>,
}

impl Metadata {
    /// Create new empty metadata
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set a metadata field
    pub fn set<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.fields.insert(key.into(), value.into());
    }
    
    /// Get a metadata field
    pub fn get(&self, key: &str) -> Option<&String> {
        self.fields.get(key)
    }
    
    /// Check if a field exists
    pub fn has(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }
    
    /// Remove a field
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.fields.remove(key)
    }
    
    /// Get all field names
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.fields.keys()
    }
    
    /// Check if metadata is empty
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_language_pair() {
        let pair = LanguagePair::from_codes("en", "es");
        assert_eq!(pair.source, "en");
        assert_eq!(pair.target, "es");
        assert!(pair.is_valid());
        
        let reverse = pair.reverse();
        assert_eq!(reverse.source, "es");
        assert_eq!(reverse.target, "en");
        
        let invalid = LanguagePair::from_codes("en", "en");
        assert!(!invalid.is_valid());
    }
    
    #[test]
    fn test_language_codes() {
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::Spanish.code(), "es");
        assert_eq!(Language::French.code(), "fr");
        
        assert_eq!(Language::from_code("en"), Some(Language::English));
        assert_eq!(Language::from_code("unknown"), Some(Language::Custom("unknown".to_string())));
    }
    
    #[test]
    fn test_translation_status_workflow() {
        let status = TranslationStatus::NotStarted;
        assert!(status.allows_editing());
        assert!(!status.requires_review());

        let next_statuses = status.next_statuses();
        assert_eq!(next_statuses, vec![TranslationStatus::InProgress]);

        let in_progress = TranslationStatus::InProgress;
        assert!(in_progress.allows_editing());
        assert!(!in_progress.requires_review());

        let pending_review = TranslationStatus::PendingReview;
        assert!(!pending_review.allows_editing());
        assert!(pending_review.requires_review());

        let published = TranslationStatus::Published;
        assert!(!published.allows_editing());
        assert!(published.next_statuses().is_empty());
    }
    
    #[test]
    fn test_quality_levels() {
        assert!(Quality::Published > Quality::Approved);
        assert!(Quality::Approved > Quality::Review);
        assert!(Quality::Review > Quality::Draft);
        
        assert_eq!(Quality::Published.score(), 100);
        assert_eq!(Quality::Draft.score(), 25);
    }
    
    #[test]
    fn test_metadata() {
        let mut metadata = Metadata::new();
        assert!(metadata.is_empty());
        
        metadata.set("author", "John Doe");
        metadata.set("version", "1.0");
        
        assert!(!metadata.is_empty());
        assert!(metadata.has("author"));
        assert_eq!(metadata.get("author"), Some(&"John Doe".to_string()));
        
        let removed = metadata.remove("version");
        assert_eq!(removed, Some("1.0".to_string()));
        assert!(!metadata.has("version"));
    }
    
    #[test]
    fn test_domain_descriptions() {
        assert_eq!(Domain::Technical.description(), "Technical documentation");
        assert_eq!(Domain::Medical.description(), "Medical and healthcare");
        assert_eq!(Domain::Legal.description(), "Legal documents");
    }
}