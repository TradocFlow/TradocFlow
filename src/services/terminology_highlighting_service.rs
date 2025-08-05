use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::models::translation_models::Term;
use crate::services::terminology_service::TerminologyService;

/// Represents a highlighted term in text with position and styling information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermHighlight {
    pub term_id: Uuid,
    pub term: String,
    pub start_position: usize,
    pub end_position: usize,
    pub highlight_type: HighlightType,
    pub definition: Option<String>,
    pub confidence: f32,
}

/// Types of terminology highlighting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HighlightType {
    DoNotTranslate,      // Terms that should not be translated
    Inconsistent,        // Terms used inconsistently across languages
    Suggestion,          // Suggested terminology
    Validated,           // Validated terminology usage
}

/// Terminology consistency check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyCheckResult {
    pub term: String,
    pub inconsistencies: Vec<LanguageInconsistency>,
    pub suggestions: Vec<String>,
}

/// Language-specific inconsistency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInconsistency {
    pub language: String,
    pub expected_term: String,
    pub found_terms: Vec<String>,
    pub positions: Vec<usize>,
}

/// Real-time terminology suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminologySuggestion {
    pub original_text: String,
    pub suggested_term: String,
    pub definition: Option<String>,
    pub confidence: f32,
    pub position: usize,
    pub reason: String,
}

/// Service for real-time terminology highlighting and validation
pub struct TerminologyHighlightingService {
    terminology_service: Arc<TerminologyService>,
    term_cache: Arc<Mutex<HashMap<Uuid, Vec<Term>>>>,
    regex_cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl TerminologyHighlightingService {
    pub fn new(terminology_service: Arc<TerminologyService>) -> Self {
        Self {
            terminology_service,
            term_cache: Arc::new(Mutex::new(HashMap::new())),
            regex_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Analyze text and return highlighted terms with their positions
    pub async fn highlight_terms_in_text(
        &self,
        text: &str,
        project_id: Uuid,
        language: &str,
    ) -> Result<Vec<TermHighlight>> {
        let terms = self.get_cached_terms(project_id).await?;
        let mut highlights = Vec::new();

        for term in &terms {
            let term_highlights = self.find_term_occurrences(text, term, language).await?;
            highlights.extend(term_highlights);
        }

        // Sort highlights by position
        highlights.sort_by(|a, b| a.start_position.cmp(&b.start_position));

        Ok(highlights)
    }

    /// Find all occurrences of a specific term in text
    async fn find_term_occurrences(
        &self,
        text: &str,
        term: &Term,
        language: &str,
    ) -> Result<Vec<TermHighlight>> {
        let mut highlights = Vec::new();
        
        // Create regex for word boundary matching
        let pattern = format!(r"\b{}\b", regex::escape(&term.term));
        let regex = self.get_cached_regex(&pattern)?;

        for mat in regex.find_iter(text) {
            let highlight_type = if term.do_not_translate {
                HighlightType::DoNotTranslate
            } else {
                HighlightType::Validated
            };

            highlights.push(TermHighlight {
                term_id: term.id,
                term: term.term.clone(),
                start_position: mat.start(),
                end_position: mat.end(),
                highlight_type,
                definition: term.definition.clone(),
                confidence: 1.0, // Exact match has full confidence
            });
        }

        Ok(highlights)
    }

    /// Check terminology consistency across multiple languages
    pub async fn check_consistency_across_languages(
        &self,
        texts: HashMap<String, String>, // language -> text
        project_id: Uuid,
    ) -> Result<Vec<ConsistencyCheckResult>> {
        let terms = self.get_cached_terms(project_id).await?;
        let mut consistency_results = Vec::new();

        for term in &terms {
            if !term.do_not_translate {
                continue; // Only check consistency for translatable terms
            }

            let mut inconsistencies = Vec::new();

            for (language, text) in &texts {
                let found_variations = self.find_term_variations(text, &term.term).await?;
                
                if !found_variations.is_empty() && !found_variations.contains(&term.term) {
                    inconsistencies.push(LanguageInconsistency {
                        language: language.clone(),
                        expected_term: term.term.clone(),
                        found_terms: found_variations.clone(),
                        positions: self.find_variation_positions(text, &found_variations).await?,
                    });
                }
            }

            if !inconsistencies.is_empty() {
                consistency_results.push(ConsistencyCheckResult {
                    term: term.term.clone(),
                    inconsistencies,
                    suggestions: vec![term.term.clone()], // Suggest the canonical term
                });
            }
        }

        Ok(consistency_results)
    }

    /// Generate terminology suggestions for text
    pub async fn generate_terminology_suggestions(
        &self,
        text: &str,
        project_id: Uuid,
        language: &str,
    ) -> Result<Vec<TerminologySuggestion>> {
        let terms = self.get_cached_terms(project_id).await?;
        let mut suggestions = Vec::new();

        // Look for potential terminology that could be standardized
        let words = self.extract_significant_words(text);

        for word in words {
            // Check if this word is similar to any existing terms
            for term in &terms {
                let similarity = self.calculate_similarity(&word.text, &term.term);
                
                if similarity > 0.7 && similarity < 1.0 {
                    suggestions.push(TerminologySuggestion {
                        original_text: word.text.clone(),
                        suggested_term: term.term.clone(),
                        definition: term.definition.clone(),
                        confidence: similarity,
                        position: word.position,
                        reason: format!("Similar to existing term '{}' ({}% match)", 
                                      term.term, (similarity * 100.0) as u32),
                    });
                }
            }
        }

        // Sort by confidence
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        Ok(suggestions)
    }

    /// Update highlighting when text changes in real-time
    pub async fn update_highlighting_for_text_change(
        &self,
        text: &str,
        change_start: usize,
        change_end: usize,
        project_id: Uuid,
        language: &str,
    ) -> Result<Vec<TermHighlight>> {
        // For real-time updates, we only need to re-analyze the changed region
        // plus some context around it to catch word boundaries
        
        let context_start = change_start.saturating_sub(50);
        let context_end = std::cmp::min(change_end + 50, text.len());
        
        let context_text = &text[context_start..context_end];
        let mut highlights = self.highlight_terms_in_text(context_text, project_id, language).await?;
        
        // Adjust positions to account for the context offset
        for highlight in &mut highlights {
            highlight.start_position += context_start;
            highlight.end_position += context_start;
        }
        
        Ok(highlights)
    }

    /// Get cached terms for a project, refreshing if necessary
    async fn get_cached_terms(&self, project_id: Uuid) -> Result<Vec<Term>> {
        let mut cache = self.term_cache.lock().unwrap();
        
        if !cache.contains_key(&project_id) {
            let terms = self.terminology_service
                .get_non_translatable_terms(project_id)
                .await?;
            cache.insert(project_id, terms);
        }
        
        Ok(cache.get(&project_id).unwrap().clone())
    }

    /// Get cached regex, creating if necessary
    fn get_cached_regex(&self, pattern: &str) -> Result<Regex> {
        let mut cache = self.regex_cache.lock().unwrap();
        
        if !cache.contains_key(pattern) {
            let regex = Regex::new(pattern)?;
            cache.insert(pattern.to_string(), regex);
        }
        
        Ok(cache.get(pattern).unwrap().clone())
    }

    /// Find variations of a term in text (case variations, plurals, etc.)
    async fn find_term_variations(&self, text: &str, term: &str) -> Result<Vec<String>> {
        let mut variations = Vec::new();
        
        // Case variations
        let term_lower = term.to_lowercase();
        let term_upper = term.to_uppercase();
        let term_title = self.to_title_case(term);
        
        let patterns = vec![
            term.to_string(),
            term_lower,
            term_upper,
            term_title,
            format!("{}s", term), // Simple plural
            format!("{}es", term), // Plural with 'es'
        ];
        
        for pattern in patterns {
            let regex_pattern = format!(r"\b{}\b", regex::escape(&pattern));
            if let Ok(regex) = Regex::new(&regex_pattern) {
                for mat in regex.find_iter(text) {
                    let found = mat.as_str().to_string();
                    if !variations.contains(&found) {
                        variations.push(found);
                    }
                }
            }
        }
        
        Ok(variations)
    }

    /// Find positions of term variations in text
    async fn find_variation_positions(&self, text: &str, variations: &[String]) -> Result<Vec<usize>> {
        let mut positions = Vec::new();
        
        for variation in variations {
            let pattern = format!(r"\b{}\b", regex::escape(variation));
            if let Ok(regex) = Regex::new(&pattern) {
                for mat in regex.find_iter(text) {
                    positions.push(mat.start());
                }
            }
        }
        
        positions.sort();
        positions.dedup();
        
        Ok(positions)
    }

    /// Extract significant words from text for terminology analysis
    fn extract_significant_words(&self, text: &str) -> Vec<SignificantWord> {
        let mut words = Vec::new();
        
        // Simple word extraction - in a real implementation, this could be more sophisticated
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

    /// Check if a word is a common word that shouldn't be considered for terminology
    fn is_common_word(&self, word: &str) -> bool {
        let common_words = [
            "the", "and", "for", "are", "but", "not", "you", "all", "can", "had", "her", "was", "one", "our", "out", "day", "get", "has", "him", "his", "how", "man", "new", "now", "old", "see", "two", "way", "who", "boy", "did", "its", "let", "put", "say", "she", "too", "use"
        ];
        
        common_words.contains(&word.to_lowercase().as_str())
    }

    /// Calculate similarity between two strings using Levenshtein distance
    fn calculate_similarity(&self, s1: &str, s2: &str) -> f32 {
        let len1 = s1.len();
        let len2 = s2.len();
        
        if len1 == 0 && len2 == 0 {
            return 1.0;
        }
        
        if len1 == 0 || len2 == 0 {
            return 0.0;
        }
        
        let max_len = std::cmp::max(len1, len2);
        let distance = self.levenshtein_distance(s1, s2);
        
        1.0 - (distance as f32 / max_len as f32)
    }

    /// Calculate Levenshtein distance between two strings
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

    /// Convert string to title case
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

    /// Invalidate term cache for a project (call when terms are updated)
    pub fn invalidate_cache(&self, project_id: Uuid) {
        let mut cache = self.term_cache.lock().unwrap();
        cache.remove(&project_id);
    }
}

/// Represents a significant word found in text
#[derive(Debug, Clone)]
struct SignificantWord {
    text: String,
    position: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use chrono::Utc;

    /// Helper function to create a test terminology service with sample data
    async fn create_test_terminology_service() -> (Arc<TerminologyService>, Uuid, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let service = Arc::new(TerminologyService::new(temp_dir.path().to_path_buf()).unwrap());
        let project_id = Uuid::new_v4();
        
        // Add some test terms
        let terms = vec![
            crate::models::translation_models::Term {
                id: Uuid::new_v4(),
                term: "API".to_string(),
                definition: Some("Application Programming Interface".to_string()),
                do_not_translate: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            crate::models::translation_models::Term {
                id: Uuid::new_v4(),
                term: "database".to_string(),
                definition: Some("A structured collection of data".to_string()),
                do_not_translate: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            crate::models::translation_models::Term {
                id: Uuid::new_v4(),
                term: "JSON".to_string(),
                definition: Some("JavaScript Object Notation".to_string()),
                do_not_translate: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];
        
        for term in terms {
            service.update_terminology(project_id, vec![term]).await.unwrap();
        }
        
        (service, project_id, temp_dir)
    }

    #[tokio::test]
    async fn test_highlight_terms_in_text() {
        let (terminology_service, project_id, _temp_dir) = create_test_terminology_service().await;
        let highlighting_service = TerminologyHighlightingService::new(terminology_service);
        
        let text = "The API uses JSON format to communicate with the database.";
        let highlights = highlighting_service
            .highlight_terms_in_text(text, project_id, "en")
            .await
            .unwrap();
        
        // Should find API, JSON, and database
        assert!(highlights.len() >= 3);
        
        // Check that API is marked as do not translate
        let api_highlight = highlights.iter().find(|h| h.term == "API").unwrap();
        assert!(matches!(api_highlight.highlight_type, HighlightType::DoNotTranslate));
        assert_eq!(api_highlight.start_position, 4);
        assert_eq!(api_highlight.end_position, 7);
    }

    #[test]
    fn test_calculate_similarity() {
        let temp_dir = TempDir::new().unwrap();
        let service = TerminologyHighlightingService::new(
            Arc::new(TerminologyService::new(temp_dir.path().to_path_buf()).unwrap())
        );
        
        assert_eq!(service.calculate_similarity("hello", "hello"), 1.0);
        assert_eq!(service.calculate_similarity("hello", "hallo"), 0.8);
        assert!(service.calculate_similarity("hello", "world") < 0.5);
    }

    #[test]
    fn test_levenshtein_distance() {
        let temp_dir = TempDir::new().unwrap();
        let service = TerminologyHighlightingService::new(
            Arc::new(TerminologyService::new(temp_dir.path().to_path_buf()).unwrap())
        );
        
        assert_eq!(service.levenshtein_distance("hello", "hello"), 0);
        assert_eq!(service.levenshtein_distance("hello", "hallo"), 1);
        assert_eq!(service.levenshtein_distance("hello", "world"), 4);
    }
}