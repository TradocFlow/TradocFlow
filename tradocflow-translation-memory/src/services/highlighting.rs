//! Real-time terminology highlighting service for text analysis

use crate::error::{Result, TranslationMemoryError};
use crate::models::{Terminology, Language};
use crate::services::terminology::TerminologyService;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;
use regex::Regex;

/// Represents a highlighted term in text with position and styling information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TermHighlight {
    pub term_id: Uuid,
    pub term: String,
    pub start_position: usize,
    pub end_position: usize,
    pub highlight_type: HighlightType,
    pub definition: Option<String>,
    pub confidence: f32,
    pub context: Option<String>,
    pub language: Language,
}

/// Types of terminology highlighting
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum HighlightType {
    /// Terms that should not be translated
    DoNotTranslate,
    /// Terms used inconsistently across languages
    Inconsistent,
    /// Suggested terminology
    Suggestion,
    /// Validated terminology usage
    Validated,
    /// Terms with low confidence matches
    LowConfidence,
    /// Terms that are contextually relevant
    Contextual,
}

/// Terminology consistency check result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConsistencyCheckResult {
    pub term: String,
    pub inconsistencies: Vec<LanguageInconsistency>,
    pub suggestions: Vec<String>,
    pub severity: ConsistencySeverity,
}

/// Severity of consistency issues
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ConsistencySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Language-specific inconsistency information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanguageInconsistency {
    pub language: Language,
    pub expected_term: String,
    pub found_terms: Vec<String>,
    pub positions: Vec<usize>,
    pub confidence: f32,
}

/// Real-time terminology suggestion
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminologySuggestion {
    pub original_text: String,
    pub suggested_term: String,
    pub definition: Option<String>,
    pub confidence: f32,
    pub position: usize,
    pub reason: String,
    pub highlight_type: HighlightType,
}

/// Highlighting configuration options
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HighlightingConfig {
    pub case_sensitive: bool,
    pub word_boundaries_only: bool,
    pub min_confidence_threshold: f32,
    pub max_context_length: usize,
    pub highlight_overlaps: bool,
    pub include_variations: bool,
}

impl Default for HighlightingConfig {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            word_boundaries_only: true,
            min_confidence_threshold: 0.6,
            max_context_length: 100,
            highlight_overlaps: false,
            include_variations: true,
        }
    }
}

/// Cache for regex patterns and term data
#[derive(Debug, Default)]
struct HighlightingCache {
    regex_patterns: HashMap<String, Regex>,
    term_cache: HashMap<Uuid, Vec<Terminology>>,
    highlight_cache: HashMap<String, Vec<TermHighlight>>,
    last_updated: Option<chrono::DateTime<chrono::Utc>>,
}

/// Thread-safe highlighting service with real-time text analysis
#[derive(Debug)]
pub struct HighlightingService {
    terminology_service: Arc<TerminologyService>,
    cache: Arc<RwLock<HighlightingCache>>,
    config: HighlightingConfig,
}

impl HighlightingService {
    /// Create a new highlighting service
    pub async fn new(
        terminology_service: Arc<TerminologyService>,
        config: Option<HighlightingConfig>,
    ) -> Result<Self> {
        Ok(Self {
            terminology_service,
            cache: Arc::new(RwLock::new(HighlightingCache::default())),
            config: config.unwrap_or_default(),
        })
    }
    
    /// Analyze text and return highlighted terms with their positions
    pub async fn highlight_terms_in_text(
        &self,
        text: &str,
        project_id: Uuid,
        language: Language,
    ) -> Result<Vec<TermHighlight>> {
        // Check cache first
        let cache_key = self.create_cache_key(text, project_id, &language);
        {
            let cache = self.cache.read().await;
            if let Some(cached_highlights) = cache.highlight_cache.get(&cache_key) {
                return Ok(cached_highlights.clone());
            }
        }
        
        // Get terms for the project
        let terms = self.get_cached_terms(project_id).await?;
        let mut highlights = Vec::new();
        
        for term in &terms {
            let term_highlights = self.find_term_occurrences(text, term, &language).await?;
            highlights.extend(term_highlights);
        }
        
        // Remove overlapping highlights if configured
        if !self.config.highlight_overlaps {
            highlights = self.remove_overlapping_highlights(highlights);
        }
        
        // Sort highlights by position
        highlights.sort_by(|a, b| a.start_position.cmp(&b.start_position));
        
        // Cache the results
        {
            let mut cache = self.cache.write().await;
            cache.highlight_cache.insert(cache_key, highlights.clone());
            cache.last_updated = Some(chrono::Utc::now());
        }
        
        Ok(highlights)
    }
    
    /// Find all occurrences of a specific term in text with context awareness
    async fn find_term_occurrences(
        &self,
        text: &str,
        term: &Terminology,
        language: &Language,
    ) -> Result<Vec<TermHighlight>> {
        let mut highlights = Vec::new();
        
        // Create base patterns to search for
        let mut search_patterns = vec![term.term.clone()];
        
        // Add variations if configured
        if self.config.include_variations {
            search_patterns.extend(self.generate_term_variations(&term.term));
        }
        
        for pattern in search_patterns {
            let regex = self.get_or_create_regex(&pattern).await?;
            
            for mat in regex.find_iter(text) {
                let confidence = self.calculate_match_confidence(&term.term, &pattern, text, mat.start());
                
                // Skip low confidence matches
                if confidence < self.config.min_confidence_threshold {
                    continue;
                }
                
                let highlight_type = self.determine_highlight_type(term, confidence);
                let context = if self.config.max_context_length > 0 {
                    Some(self.extract_context(text, mat.start(), mat.end(), self.config.max_context_length))
                } else {
                    None
                };
                
                highlights.push(TermHighlight {
                    term_id: term.id,
                    term: term.term.clone(),
                    start_position: mat.start(),
                    end_position: mat.end(),
                    highlight_type,
                    definition: term.definition.clone(),
                    confidence,
                    context,
                    language: language.clone(),
                });
            }
        }
        
        Ok(highlights)
    }
    
    /// Check terminology consistency across multiple languages
    pub async fn check_consistency_across_languages(
        &self,
        texts: HashMap<Language, String>,
        project_id: Uuid,
    ) -> Result<Vec<ConsistencyCheckResult>> {
        let terms = self.get_cached_terms(project_id).await?;
        let mut consistency_results = Vec::new();
        
        for term in &terms {
            // Only check consistency for non-translatable terms and high-importance terms
            if !term.do_not_translate && !self.is_high_importance_term(term) {
                continue;
            }
            
            let mut inconsistencies = Vec::new();
            
            for (language, text) in &texts {
                let found_variations = self.find_term_variations(text, &term.term).await?;
                
                if !found_variations.is_empty() {
                    // Check if the canonical term is present
                    let canonical_found = found_variations.iter().any(|v| 
                        if self.config.case_sensitive {
                            v == &term.term
                        } else {
                            v.to_lowercase() == term.term.to_lowercase()
                        }
                    );
                    
                    if !canonical_found || found_variations.len() > 1 {
                        let positions = self.find_variation_positions(text, &found_variations).await?;
                        let confidence = self.calculate_consistency_confidence(&found_variations, &term.term);
                        
                        inconsistencies.push(LanguageInconsistency {
                            language: language.clone(),
                            expected_term: term.term.clone(),
                            found_terms: found_variations,
                            positions,
                            confidence,
                        });
                    }
                }
            }
            
            if !inconsistencies.is_empty() {
                let severity = self.calculate_consistency_severity(&inconsistencies, term);
                consistency_results.push(ConsistencyCheckResult {
                    term: term.term.clone(),
                    inconsistencies,
                    suggestions: vec![term.term.clone()],
                    severity,
                });
            }
        }
        
        // Sort by severity
        consistency_results.sort_by(|a, b| match (&a.severity, &b.severity) {
            (ConsistencySeverity::Critical, _) => std::cmp::Ordering::Less,
            (_, ConsistencySeverity::Critical) => std::cmp::Ordering::Greater,
            (ConsistencySeverity::High, ConsistencySeverity::Low | ConsistencySeverity::Medium) => std::cmp::Ordering::Less,
            (ConsistencySeverity::Low | ConsistencySeverity::Medium, ConsistencySeverity::High) => std::cmp::Ordering::Greater,
            (ConsistencySeverity::Medium, ConsistencySeverity::Low) => std::cmp::Ordering::Less,
            (ConsistencySeverity::Low, ConsistencySeverity::Medium) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });
        
        Ok(consistency_results)
    }
    
    /// Generate terminology suggestions for text improvement
    pub async fn generate_terminology_suggestions(
        &self,
        text: &str,
        project_id: Uuid,
        _language: Language,
    ) -> Result<Vec<TerminologySuggestion>> {
        let terms = self.get_cached_terms(project_id).await?;
        let mut suggestions = Vec::new();
        
        // Extract significant words from text for analysis
        let words = self.extract_significant_words(text);
        
        for word in words {
            // Check if this word is similar to any existing terms
            for term in &terms {
                let similarity = self.calculate_text_similarity(&word.text, &term.term);
                
                if similarity > 0.7 && similarity < 1.0 {
                    let highlight_type = if term.do_not_translate {
                        HighlightType::DoNotTranslate
                    } else {
                        HighlightType::Suggestion
                    };
                    
                    suggestions.push(TerminologySuggestion {
                        original_text: word.text.clone(),
                        suggested_term: term.term.clone(),
                        definition: term.definition.clone(),
                        confidence: similarity,
                        position: word.position,
                        reason: format!("Similar to existing term '{}' ({}% match)", 
                                      term.term, (similarity * 100.0) as u32),
                        highlight_type,
                    });
                }
            }
        }
        
        // Sort by confidence and remove duplicates
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.dedup_by(|a, b| a.original_text == b.original_text && a.suggested_term == b.suggested_term);
        
        // Limit to top suggestions
        suggestions.truncate(20);
        
        Ok(suggestions)
    }
    
    /// Update highlighting when text changes in real-time
    pub async fn update_highlighting_for_text_change(
        &self,
        text: &str,
        change_start: usize,
        change_end: usize,
        project_id: Uuid,
        language: Language,
    ) -> Result<Vec<TermHighlight>> {
        // For real-time updates, analyze only the changed region plus context
        let context_padding = 100; // characters
        let context_start = change_start.saturating_sub(context_padding);
        let context_end = std::cmp::min(change_end + context_padding, text.len());
        
        let context_text = &text[context_start..context_end];
        let mut highlights = self.highlight_terms_in_text(context_text, project_id, language).await?;
        
        // Adjust positions to account for the context offset
        for highlight in &mut highlights {
            highlight.start_position += context_start;
            highlight.end_position += context_start;
            
            // Filter out highlights that are outside the affected area
            if highlight.end_position < change_start - context_padding || 
               highlight.start_position > change_end + context_padding {
                continue;
            }
        }
        
        // Remove highlights that are now outside the text bounds
        highlights.retain(|h| h.end_position <= text.len());
        
        Ok(highlights)
    }
    
    /// Clear cache for a specific project
    pub async fn invalidate_project_cache(&self, project_id: Uuid) {
        let mut cache = self.cache.write().await;
        
        // Remove project-specific cached terms
        cache.term_cache.remove(&project_id);
        
        // Remove highlight cache entries for this project
        let project_prefix = format!("{}:", project_id);
        cache.highlight_cache.retain(|key, _| !key.starts_with(&project_prefix));
        
        cache.last_updated = Some(chrono::Utc::now());
    }
    
    /// Get cache statistics for monitoring
    pub async fn get_cache_stats(&self) -> (usize, usize, usize, Option<chrono::DateTime<chrono::Utc>>) {
        let cache = self.cache.read().await;
        (
            cache.regex_patterns.len(),
            cache.term_cache.len(),
            cache.highlight_cache.len(),
            cache.last_updated,
        )
    }
    
    /// Invalidate cache for a specific term
    pub async fn invalidate_term_cache(&self, term_id: uuid::Uuid) {
        let mut cache = self.cache.write().await;
        
        // Remove term-specific cache entries
        cache.term_cache.retain(|_, terms| {
            terms.retain(|t| t.id != term_id);
            !terms.is_empty()
        });
        
        // Clear highlight cache as it may contain stale data
        cache.highlight_cache.clear();
        
        cache.last_updated = Some(chrono::Utc::now());
    }
    
    /// Clear all caches
    pub async fn clear_all_caches(&self) {
        let mut cache = self.cache.write().await;
        cache.regex_patterns.clear();
        cache.term_cache.clear();
        cache.highlight_cache.clear();
        cache.last_updated = Some(chrono::Utc::now());
    }
    
    // Private helper methods
    
    async fn get_cached_terms(&self, project_id: Uuid) -> Result<Vec<Terminology>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_terms) = cache.term_cache.get(&project_id) {
                return Ok(cached_terms.clone());
            }
        }
        
        // Fetch from terminology service
        let terms = self.terminology_service.get_terms_by_project(project_id).await?;
        
        // Cache the results
        {
            let mut cache = self.cache.write().await;
            cache.term_cache.insert(project_id, terms.clone());
        }
        
        Ok(terms)
    }
    
    async fn get_or_create_regex(&self, pattern: &str) -> Result<Regex> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(regex) = cache.regex_patterns.get(pattern) {
                return Ok(regex.clone());
            }
        }
        
        // Create regex pattern
        let regex_pattern = if self.config.word_boundaries_only {
            format!(r"\b{}\b", regex::escape(pattern))
        } else {
            regex::escape(pattern)
        };
        
        let regex = if self.config.case_sensitive {
            Regex::new(&regex_pattern)
        } else {
            Regex::new(&format!("(?i){}", regex_pattern))
        }.map_err(|e| TranslationMemoryError::ValidationError(format!("Invalid regex pattern: {}", e)))?;
        
        // Cache the regex
        {
            let mut cache = self.cache.write().await;
            cache.regex_patterns.insert(pattern.to_string(), regex.clone());
        }
        
        Ok(regex)
    }
    
    fn generate_term_variations(&self, term: &str) -> Vec<String> {
        let mut variations = Vec::new();
        
        // Case variations
        variations.push(term.to_lowercase());
        variations.push(term.to_uppercase());
        variations.push(self.to_title_case(term));
        
        // Simple morphological variations (plurals, etc.)
        variations.push(format!("{}s", term));
        variations.push(format!("{}es", term));
        variations.push(format!("{}ing", term));
        variations.push(format!("{}ed", term));
        
        // Remove duplicates and the original term
        variations.retain(|v| v != term);
        variations.sort();
        variations.dedup();
        
        variations
    }
    
    fn calculate_match_confidence(&self, original_term: &str, matched_pattern: &str, text: &str, position: usize) -> f32 {
        let mut confidence: f32 = 1.0;
        
        // Reduce confidence for non-exact matches
        if original_term != matched_pattern {
            confidence *= 0.8;
        }
        
        // Check context for additional confidence
        let context = self.extract_context(text, position, position + matched_pattern.len(), 50);
        
        // Higher confidence if surrounded by punctuation or whitespace
        let before_char = if position > 0 { 
            text.chars().nth(position - 1) 
        } else { 
            None 
        };
        let after_char = text.chars().nth(position + matched_pattern.len());
        
        if let (Some(before), Some(after)) = (before_char, after_char) {
            if before.is_whitespace() && after.is_whitespace() {
                confidence *= 1.1;
            } else if before.is_ascii_punctuation() || after.is_ascii_punctuation() {
                confidence *= 1.05;
            }
        }
        
        // Contextual confidence based on surrounding words
        if context.to_lowercase().contains("technical") || context.to_lowercase().contains("specific") {
            confidence *= 1.05;
        }
        
        confidence.min(1.0)
    }
    
    fn determine_highlight_type(&self, term: &Terminology, confidence: f32) -> HighlightType {
        if term.do_not_translate {
            HighlightType::DoNotTranslate
        } else if confidence >= 0.9 {
            HighlightType::Validated
        } else if confidence >= 0.7 {
            HighlightType::Contextual
        } else if confidence >= self.config.min_confidence_threshold {
            HighlightType::LowConfidence
        } else {
            HighlightType::Suggestion
        }
    }
    
    fn remove_overlapping_highlights(&self, mut highlights: Vec<TermHighlight>) -> Vec<TermHighlight> {
        highlights.sort_by(|a, b| {
            a.start_position.cmp(&b.start_position)
                .then(b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
        });
        
        let mut result = Vec::new();
        let mut last_end = 0;
        
        for highlight in highlights {
            if highlight.start_position >= last_end {
                last_end = highlight.end_position;
                result.push(highlight);
            }
        }
        
        result
    }
    
    async fn find_term_variations(&self, text: &str, term: &str) -> Result<Vec<String>> {
        let variations = self.generate_term_variations(term);
        let mut found_variations = Vec::new();
        
        for variation in variations {
            let regex = self.get_or_create_regex(&variation).await?;
            if regex.is_match(text) {
                found_variations.push(variation);
            }
        }
        
        // Also check for the original term
        let regex = self.get_or_create_regex(term).await?;
        if regex.is_match(text) {
            found_variations.push(term.to_string());
        }
        
        found_variations.sort();
        found_variations.dedup();
        Ok(found_variations)
    }
    
    async fn find_variation_positions(&self, text: &str, variations: &[String]) -> Result<Vec<usize>> {
        let mut positions = Vec::new();
        
        for variation in variations {
            let regex = self.get_or_create_regex(variation).await?;
            for mat in regex.find_iter(text) {
                positions.push(mat.start());
            }
        }
        
        positions.sort();
        positions.dedup();
        Ok(positions)
    }
    
    fn calculate_consistency_confidence(&self, found_variations: &[String], canonical_term: &str) -> f32 {
        if found_variations.len() == 1 && found_variations[0] == canonical_term {
            1.0
        } else if found_variations.contains(&canonical_term.to_string()) {
            0.7
        } else {
            0.3
        }
    }
    
    fn calculate_consistency_severity(&self, inconsistencies: &[LanguageInconsistency], term: &Terminology) -> ConsistencySeverity {
        let avg_confidence: f32 = inconsistencies.iter().map(|i| i.confidence).sum::<f32>() / inconsistencies.len() as f32;
        let language_count = inconsistencies.len();
        
        if term.do_not_translate && avg_confidence < 0.5 {
            ConsistencySeverity::Critical
        } else if language_count > 3 && avg_confidence < 0.6 {
            ConsistencySeverity::High
        } else if language_count > 1 && avg_confidence < 0.7 {
            ConsistencySeverity::Medium
        } else {
            ConsistencySeverity::Low
        }
    }
    
    fn is_high_importance_term(&self, term: &Terminology) -> bool {
        // Terms with definitions are considered more important
        if term.definition.is_some() {
            return true;
        }
        
        // Technical terms or acronyms
        if term.term.chars().all(|c| c.is_ascii_uppercase()) {
            return true;
        }
        
        // Terms that are likely technical or domain-specific
        let technical_indicators = ["api", "json", "xml", "http", "url", "id", "uuid"];
        let term_lower = term.term.to_lowercase();
        
        technical_indicators.iter().any(|&indicator| term_lower.contains(indicator))
    }
    
    fn extract_significant_words(&self, text: &str) -> Vec<SignificantWord> {
        let mut words = Vec::new();
        let word_regex = Regex::new(r"\b[A-Za-z]{3,}\b").unwrap();
        
        for mat in word_regex.find_iter(text) {
            let word = mat.as_str().to_string();
            
            // Skip common words
            if !self.is_common_word(&word) {
                words.push(SignificantWord {
                    text: word,
                    position: mat.start(),
                });
            }
        }
        
        words
    }
    
    fn is_common_word(&self, word: &str) -> bool {
        let common_words = [
            "the", "and", "for", "are", "but", "not", "you", "all", "can", "had", 
            "her", "was", "one", "our", "out", "day", "get", "has", "him", "his", 
            "how", "man", "new", "now", "old", "see", "two", "way", "who", "boy", 
            "did", "its", "let", "put", "say", "she", "too", "use", "any", "may",
            "will", "would", "could", "should", "much", "very", "well", "good", 
            "great", "little", "long", "first", "last", "next", "other", "same",
            "different", "important", "large", "small", "right", "left", "high",
            "low", "early", "late", "young", "old", "strong", "weak"
        ];
        
        common_words.contains(&word.to_lowercase().as_str())
    }
    
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f32 {
        let len1 = text1.len();
        let len2 = text2.len();
        
        if len1 == 0 && len2 == 0 {
            return 1.0;
        }
        
        if len1 == 0 || len2 == 0 {
            return 0.0;
        }
        
        let max_len = std::cmp::max(len1, len2);
        let distance = self.levenshtein_distance(text1, text2);
        
        1.0 - (distance as f32 / max_len as f32)
    }
    
    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();
        let len1 = chars1.len();
        let len2 = chars2.len();
        
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
        
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        
        for j in 0..=len2 {
            matrix[0][j] = j;
        }
        
        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
                matrix[i][j] = std::cmp::min(
                    std::cmp::min(
                        matrix[i - 1][j] + 1,      // deletion
                        matrix[i][j - 1] + 1,      // insertion
                    ),
                    matrix[i - 1][j - 1] + cost,   // substitution
                );
            }
        }
        
        matrix[len1][len2]
    }
    
    fn extract_context(&self, text: &str, start: usize, end: usize, max_length: usize) -> String {
        if max_length == 0 {
            return String::new();
        }
        
        let context_start = start.saturating_sub(max_length / 2);
        let context_end = std::cmp::min(end + max_length / 2, text.len());
        let context = &text[context_start..context_end];
        
        let mut result = String::new();
        if context_start > 0 {
            result.push_str("...");
        }
        result.push_str(context);
        if context_end < text.len() {
            result.push_str("...");
        }
        
        result
    }
    
    fn to_title_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = true;
        
        for c in s.chars() {
            if c.is_alphabetic() {
                if capitalize_next {
                    result.push(c.to_uppercase().next().unwrap());
                    capitalize_next = false;
                } else {
                    result.push(c.to_lowercase().next().unwrap());
                }
            } else {
                result.push(c);
                capitalize_next = c.is_whitespace();
            }
        }
        
        result
    }
    
    fn create_cache_key(&self, text: &str, project_id: Uuid, language: &Language) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        project_id.hash(&mut hasher);
        language.hash(&mut hasher);
        
        format!("{}:{}:{:x}", project_id, language, hasher.finish())
    }
}

/// Represents a significant word found in text
#[derive(Debug, Clone)]
struct SignificantWord {
    text: String,
    position: usize,
}