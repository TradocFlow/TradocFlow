use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tokio::time::{Duration, Instant};
use regex::Regex;
use crate::Result;

/// Language-specific sentence boundary patterns and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageProfile {
    pub language: String,
    pub sentence_boundary_patterns: Vec<String>,
    pub abbreviation_patterns: Vec<String>,
    pub average_sentence_length: f64,
    pub length_variance: f64,
    pub typical_word_count: f64,
    pub common_punctuation: Vec<char>,
}

impl LanguageProfile {
    /// Create a default English language profile
    pub fn english() -> Self {
        Self {
            language: "en".to_string(),
            sentence_boundary_patterns: vec![
                r"[.!?]+\s+[A-Z]".to_string(),
                r"[.!?]+$".to_string(),
            ],
            abbreviation_patterns: vec![
                r"\b(?:Mr|Mrs|Dr|Prof|Inc|Ltd|Corp|etc|vs|e\.g|i\.e)\.\s*".to_string(),
            ],
            average_sentence_length: 85.0,
            length_variance: 25.0,
            typical_word_count: 15.0,
            common_punctuation: vec!['.', '!', '?', ',', ';', ':'],
        }
    }

    /// Create a default Spanish language profile
    pub fn spanish() -> Self {
        Self {
            language: "es".to_string(),
            sentence_boundary_patterns: vec![
                r"[.!?]+\s+[A-ZÁÉÍÓÚÑ]".to_string(),
                r"[.!?]+$".to_string(),
            ],
            abbreviation_patterns: vec![
                r"\b(?:Sr|Sra|Dr|Prof|S\.A|S\.L|etc|p\.ej|es decir)\.\s*".to_string(),
            ],
            average_sentence_length: 95.0,
            length_variance: 30.0,
            typical_word_count: 18.0,
            common_punctuation: vec!['.', '!', '?', ',', ';', ':', '¡', '¿'],
        }
    }

    /// Create a default French language profile
    pub fn french() -> Self {
        Self {
            language: "fr".to_string(),
            sentence_boundary_patterns: vec![
                r"[.!?]+\s+[A-ZÀÂÄÉÈÊËÏÎÔÖÙÛÜÇ]".to_string(),
                r"[.!?]+$".to_string(),
            ],
            abbreviation_patterns: vec![
                r"\b(?:M|Mme|Dr|Prof|SARL|SA|etc|p\.ex|c-à-d)\.\s*".to_string(),
            ],
            average_sentence_length: 100.0,
            length_variance: 35.0,
            typical_word_count: 20.0,
            common_punctuation: vec!['.', '!', '?', ',', ';', ':'],
        }
    }

    /// Create a default German language profile
    pub fn german() -> Self {
        Self {
            language: "de".to_string(),
            sentence_boundary_patterns: vec![
                r"[.!?]+\s+[A-ZÄÖÜ]".to_string(),
                r"[.!?]+$".to_string(),
            ],
            abbreviation_patterns: vec![
                r"\b(?:Dr|Prof|GmbH|AG|etc|z\.B|d\.h)\.\s*".to_string(),
            ],
            average_sentence_length: 110.0,
            length_variance: 40.0,
            typical_word_count: 22.0,
            common_punctuation: vec!['.', '!', '?', ',', ';', ':'],
        }
    }

    /// Get language profile by language code
    pub fn for_language(language: &str) -> Self {
        match language.to_lowercase().as_str() {
            "en" => Self::english(),
            "es" => Self::spanish(),
            "fr" => Self::french(),
            "de" => Self::german(),
            _ => Self::english(), // Default fallback
        }
    }
}

/// A sentence boundary detected in text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceBoundary {
    pub start_offset: usize,
    pub end_offset: usize,
    pub text: String,
    pub confidence: f64,
    pub boundary_type: BoundaryType,
}

/// Type of sentence boundary
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BoundaryType {
    Period,
    Exclamation,
    Question,
    Ellipsis,
    EndOfParagraph,
    Custom(String),
}

/// Alignment between sentences in different languages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceAlignment {
    pub id: Uuid,
    pub source_sentence: SentenceBoundary,
    pub target_sentence: SentenceBoundary,
    pub source_language: String,
    pub target_language: String,
    pub alignment_confidence: f64,
    pub alignment_method: AlignmentMethod,
    pub validation_status: ValidationStatus,
    #[serde(skip, default = "Instant::now")]
    pub created_at: Instant,
    #[serde(skip)]
    pub last_validated: Option<Instant>,
}

/// Method used for alignment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlignmentMethod {
    PositionBased,
    LengthRatio,
    MachineLearning,
    UserValidated,
    Hybrid,
}

/// Validation status of alignment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationStatus {
    Pending,
    Validated,
    Rejected,
    NeedsReview,
}

/// Real-time alignment quality indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentQualityIndicator {
    pub overall_quality: f64,
    pub position_consistency: f64,
    pub length_ratio_consistency: f64,
    pub structural_coherence: f64,
    pub user_validation_rate: f64,
    pub problem_areas: Vec<ProblemArea>,
}

/// Areas where alignment quality is poor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemArea {
    pub start_position: usize,
    pub end_position: usize,
    pub issue_type: AlignmentIssue,
    pub severity: f64,
    pub suggestion: String,
}

/// Types of alignment issues
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlignmentIssue {
    LengthMismatch,
    StructuralDivergence,
    MissingSentence,
    ExtraSentence,
    OrderMismatch,
    BoundaryDetectionError,
}

/// Configuration for sentence alignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentConfig {
    pub max_length_ratio_deviation: f64,
    pub position_weight: f64,
    pub length_weight: f64,
    pub structure_weight: f64,
    pub confidence_threshold: f64,
    pub enable_ml_corrections: bool,
    pub auto_validation_threshold: f64,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            max_length_ratio_deviation: 2.5,
            position_weight: 0.4,
            length_weight: 0.3,
            structure_weight: 0.3,
            confidence_threshold: 0.7,
            enable_ml_corrections: true,
            auto_validation_threshold: 0.9,
        }
    }
}

/// Statistics for sentence alignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentStatistics {
    pub total_sentences: usize,
    pub aligned_sentences: usize,
    pub validated_alignments: usize,
    pub average_confidence: f64,
    pub alignment_accuracy: f64,
    pub processing_time_ms: u64,
    pub language_pair: (String, String),
}

/// Machine learning model for alignment correction
#[derive(Debug, Clone)]
pub struct AlignmentMLModel {
    // Simplified ML model - in practice would use more sophisticated approach
    pub feature_weights: HashMap<String, f64>,
    pub correction_history: VecDeque<AlignmentCorrection>,
    pub learning_rate: f64,
}

/// User correction for machine learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentCorrection {
    pub original_alignment: SentenceAlignment,
    pub corrected_alignment: SentenceAlignment,
    pub correction_reason: String,
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
}

/// Main sentence alignment service
pub struct SentenceAlignmentService {
    language_profiles: Arc<RwLock<HashMap<String, LanguageProfile>>>,
    alignment_cache: Arc<RwLock<HashMap<String, Vec<SentenceAlignment>>>>,
    ml_model: Arc<RwLock<AlignmentMLModel>>,
    config: AlignmentConfig,
    statistics: Arc<RwLock<HashMap<String, AlignmentStatistics>>>,
}

impl SentenceAlignmentService {
    /// Create a new sentence alignment service
    pub fn new(config: AlignmentConfig) -> Self {
        let mut language_profiles = HashMap::new();
        language_profiles.insert("en".to_string(), LanguageProfile::english());
        language_profiles.insert("es".to_string(), LanguageProfile::spanish());
        language_profiles.insert("fr".to_string(), LanguageProfile::french());
        language_profiles.insert("de".to_string(), LanguageProfile::german());

        let ml_model = AlignmentMLModel {
            feature_weights: [
                ("position_similarity".to_string(), 0.4),
                ("length_ratio".to_string(), 0.3),
                ("structure_similarity".to_string(), 0.2),
                ("content_similarity".to_string(), 0.1),
            ].iter().cloned().collect(),
            correction_history: VecDeque::new(),
            learning_rate: 0.01,
        };

        Self {
            language_profiles: Arc::new(RwLock::new(language_profiles)),
            alignment_cache: Arc::new(RwLock::new(HashMap::new())),
            ml_model: Arc::new(RwLock::new(ml_model)),
            config,
            statistics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Detect sentence boundaries in text
    pub async fn detect_sentence_boundaries(
        &self,
        text: &str,
        language: &str,
    ) -> Result<Vec<SentenceBoundary>> {
        let start_time = Instant::now();
        
        let profiles = self.language_profiles.read().unwrap();
        let default_profile = LanguageProfile::english();
        let profile = profiles.get(language)
            .unwrap_or(&default_profile);

        let mut boundaries = Vec::new();
        let mut current_start = 0;

        // Create regex patterns for sentence detection
        let boundary_regex = Regex::new(&profile.sentence_boundary_patterns.join("|"))
            .map_err(|e| crate::TradocumentError::Validation(format!("Regex error: {}", e)))?;
        
        let abbrev_regex = Regex::new(&profile.abbreviation_patterns.join("|"))
            .map_err(|e| crate::TradocumentError::Validation(format!("Regex error: {}", e)))?;

        // Split text into paragraphs first
        for paragraph in text.split('\n') {
            if paragraph.trim().is_empty() {
                continue;
            }

            let paragraph_start = text.find(paragraph).unwrap_or(current_start);
            let mut sentence_starts = vec![paragraph_start];

            // Find potential sentence boundaries
            for mat in boundary_regex.find_iter(paragraph) {
                let boundary_pos = paragraph_start + mat.start();
                
                // Check if this is not an abbreviation
                let text_before = &text[..boundary_pos + 1];
                if !abbrev_regex.is_match(text_before) {
                    sentence_starts.push(boundary_pos + mat.len());
                }
            }

            // Create sentence boundaries
            for i in 0..sentence_starts.len() - 1 {
                let start = sentence_starts[i];
                let end = sentence_starts[i + 1];
                let sentence_text = text[start..end].trim().to_string();

                if !sentence_text.is_empty() {
                    let boundary_type = self.detect_boundary_type(&sentence_text);
                    let confidence = self.calculate_boundary_confidence(
                        &sentence_text, 
                        profile, 
                        &boundary_type
                    );

                    boundaries.push(SentenceBoundary {
                        start_offset: start,
                        end_offset: end,
                        text: sentence_text,
                        confidence,
                        boundary_type,
                    });
                }
            }

            // Handle last sentence in paragraph
            if let Some(&last_start) = sentence_starts.last() {
                let paragraph_end = paragraph_start + paragraph.len();
                if last_start < paragraph_end {
                    let sentence_text = text[last_start..paragraph_end].trim().to_string();
                    if !sentence_text.is_empty() {
                        boundaries.push(SentenceBoundary {
                            start_offset: last_start,
                            end_offset: paragraph_end,
                            text: sentence_text,
                            confidence: 0.9, // End of paragraph is usually reliable
                            boundary_type: BoundaryType::EndOfParagraph,
                        });
                    }
                }
            }

            current_start = paragraph_start + paragraph.len() + 1;
        }

        Ok(boundaries)
    }

    /// Align sentences between two texts in different languages
    pub async fn align_sentences(
        &self,
        source_text: &str,
        target_text: &str,
        source_language: &str,
        target_language: &str,
    ) -> Result<Vec<SentenceAlignment>> {
        let start_time = Instant::now();

        // Detect sentence boundaries in both texts
        let source_sentences = self.detect_sentence_boundaries(source_text, source_language).await?;
        let target_sentences = self.detect_sentence_boundaries(target_text, target_language).await?;

        // Create cache key
        let cache_key = format!("{}:{}:{}", source_language, target_language, 
            format!("{:x}", md5::compute(format!("{}{}", source_text, target_text))));

        // Check cache first
        {
            let cache = self.alignment_cache.read().unwrap();
            if let Some(cached_alignments) = cache.get(&cache_key) {
                return Ok(cached_alignments.clone());
            }
        }

        let mut alignments: Vec<SentenceAlignment> = Vec::new();

        // Position-based alignment
        let position_alignments = self.create_position_based_alignments(
            &source_sentences, 
            &target_sentences, 
            source_language, 
            target_language
        ).await?;

        // Length ratio validation and adjustment
        let ratio_validated_alignments = self.validate_with_length_ratios(
            position_alignments, 
            source_language, 
            target_language
        ).await?;

        // Apply machine learning corrections
        let ml_corrected_alignments = if self.config.enable_ml_corrections {
            self.apply_ml_corrections(ratio_validated_alignments).await?
        } else {
            ratio_validated_alignments
        };

        // Auto-validate high-confidence alignments
        let final_alignments = self.auto_validate_alignments(ml_corrected_alignments).await?;

        // Cache the results
        {
            let mut cache = self.alignment_cache.write().unwrap();
            cache.insert(cache_key, final_alignments.clone());
        }

        // Update statistics
        self.update_statistics(
            &final_alignments, 
            source_language, 
            target_language, 
            start_time.elapsed()
        ).await?;

        Ok(final_alignments)
    }

    /// Create position-based alignments
    async fn create_position_based_alignments(
        &self,
        source_sentences: &[SentenceBoundary],
        target_sentences: &[SentenceBoundary],
        source_language: &str,
        target_language: &str,
    ) -> Result<Vec<SentenceAlignment>> {
        let mut alignments = Vec::new();
        let source_len = source_sentences.len();
        let target_len = target_sentences.len();

        if source_len == 0 || target_len == 0 {
            return Ok(alignments);
        }

        // Simple 1:1 mapping for similar length texts
        if (source_len as f64 / target_len as f64 - 1.0).abs() < 0.2 {
            for (i, source_sentence) in source_sentences.iter().enumerate() {
                if i < target_len {
                    let target_sentence = &target_sentences[i];
                    let confidence = self.calculate_alignment_confidence(
                        source_sentence, 
                        target_sentence, 
                        i as f64 / source_len as f64,
                        i as f64 / target_len as f64,
                        source_language,
                        target_language,
                    ).await?;

                    alignments.push(SentenceAlignment {
                        id: Uuid::new_v4(),
                        source_sentence: source_sentence.clone(),
                        target_sentence: target_sentence.clone(),
                        source_language: source_language.to_string(),
                        target_language: target_language.to_string(),
                        alignment_confidence: confidence,
                        alignment_method: AlignmentMethod::PositionBased,
                        validation_status: ValidationStatus::Pending,
                        created_at: Instant::now(),
                        last_validated: None,
                    });
                }
            }
        } else {
            // Dynamic programming approach for different length texts
            alignments = self.create_dynamic_alignments(
                source_sentences, 
                target_sentences, 
                source_language, 
                target_language
            ).await?;
        }

        Ok(alignments)
    }

    /// Create alignments using dynamic programming for different length texts
    async fn create_dynamic_alignments(
        &self,
        source_sentences: &[SentenceBoundary],
        target_sentences: &[SentenceBoundary],
        source_language: &str,
        target_language: &str,
    ) -> Result<Vec<SentenceAlignment>> {
        let source_len = source_sentences.len();
        let target_len = target_sentences.len();
        
        // Create scoring matrix
        let mut score_matrix = vec![vec![0.0; target_len + 1]; source_len + 1];
        let mut path_matrix = vec![vec![(0, 0); target_len + 1]; source_len + 1];

        // Fill the scoring matrix
        for i in 1..=source_len {
            for j in 1..=target_len {
                let source_sentence = &source_sentences[i - 1];
                let target_sentence = &target_sentences[j - 1];
                
                let alignment_score = self.calculate_alignment_confidence(
                    source_sentence,
                    target_sentence,
                    (i - 1) as f64 / source_len as f64,
                    (j - 1) as f64 / target_len as f64,
                    source_language,
                    target_language,
                ).await?;

                // Calculate possible moves
                let diagonal = score_matrix[i - 1][j - 1] + alignment_score;
                let up = score_matrix[i - 1][j] - 0.5; // Penalty for skipping target
                let left = score_matrix[i][j - 1] - 0.5; // Penalty for skipping source

                if diagonal >= up && diagonal >= left {
                    score_matrix[i][j] = diagonal;
                    path_matrix[i][j] = (i - 1, j - 1);
                } else if up >= left {
                    score_matrix[i][j] = up;
                    path_matrix[i][j] = (i - 1, j);
                } else {
                    score_matrix[i][j] = left;
                    path_matrix[i][j] = (i, j - 1);
                }
            }
        }

        // Backtrack to find optimal alignment path
        let mut alignments = Vec::new();
        let mut i = source_len;
        let mut j = target_len;

        while i > 0 && j > 0 {
            let (prev_i, prev_j) = path_matrix[i][j];
            
            if prev_i == i - 1 && prev_j == j - 1 {
                // Diagonal move - create alignment
                let source_sentence = &source_sentences[i - 1];
                let target_sentence = &target_sentences[j - 1];
                
                let confidence = self.calculate_alignment_confidence(
                    source_sentence,
                    target_sentence,
                    (i - 1) as f64 / source_len as f64,
                    (j - 1) as f64 / target_len as f64,
                    source_language,
                    target_language,
                ).await?;

                alignments.push(SentenceAlignment {
                    id: Uuid::new_v4(),
                    source_sentence: source_sentence.clone(),
                    target_sentence: target_sentence.clone(),
                    source_language: source_language.to_string(),
                    target_language: target_language.to_string(),
                    alignment_confidence: confidence,
                    alignment_method: AlignmentMethod::Hybrid,
                    validation_status: ValidationStatus::Pending,
                    created_at: Instant::now(),
                    last_validated: None,
                });
            }
            
            i = prev_i;
            j = prev_j;
        }

        alignments.reverse();
        Ok(alignments)
    }

    /// Validate alignments using statistical length ratios
    async fn validate_with_length_ratios(
        &self,
        alignments: Vec<SentenceAlignment>,
        source_language: &str,
        target_language: &str,
    ) -> Result<Vec<SentenceAlignment>> {
        let profiles = self.language_profiles.read().unwrap();
        let default_source_profile = LanguageProfile::english();
        let default_target_profile = LanguageProfile::english();
        let source_profile = profiles.get(source_language).unwrap_or(&default_source_profile);
        let target_profile = profiles.get(target_language).unwrap_or(&default_target_profile);

        let expected_ratio = target_profile.average_sentence_length / source_profile.average_sentence_length;
        let max_deviation = self.config.max_length_ratio_deviation;

        let mut validated_alignments = Vec::new();

        for mut alignment in alignments {
            let source_len = alignment.source_sentence.text.len() as f64;
            let target_len = alignment.target_sentence.text.len() as f64;
            let actual_ratio = target_len / source_len.max(1.0);

            let ratio_deviation = (actual_ratio / expected_ratio - 1.0).abs();
            
            if ratio_deviation <= max_deviation {
                // Length ratio is within expected bounds
                let ratio_confidence = 1.0 - (ratio_deviation / max_deviation) * 0.3;
                alignment.alignment_confidence = 
                    (alignment.alignment_confidence * 0.7) + (ratio_confidence * 0.3);
                alignment.alignment_method = AlignmentMethod::LengthRatio;
            } else {
                // Length ratio is suspicious - reduce confidence
                alignment.alignment_confidence *= 0.5;
                alignment.validation_status = ValidationStatus::NeedsReview;
            }

            validated_alignments.push(alignment);
        }

        Ok(validated_alignments)
    }

    /// Apply machine learning corrections based on user feedback
    async fn apply_ml_corrections(
        &self,
        alignments: Vec<SentenceAlignment>
    ) -> Result<Vec<SentenceAlignment>> {
        let mut ml_model = self.ml_model.write().unwrap();
        let mut corrected_alignments = Vec::new();

        for mut alignment in alignments {
            // Extract features for ML model
            let features = self.extract_alignment_features(&alignment);
            
            // Calculate ML confidence adjustment
            let ml_confidence_adjustment = self.calculate_ml_confidence(&features, &ml_model);
            
            // Apply adjustment
            alignment.alignment_confidence = 
                (alignment.alignment_confidence + ml_confidence_adjustment * 0.2).min(1.0).max(0.0);

            if alignment.alignment_confidence >= self.config.confidence_threshold {
                alignment.alignment_method = AlignmentMethod::MachineLearning;
            }

            corrected_alignments.push(alignment);
        }

        Ok(corrected_alignments)
    }

    /// Auto-validate high-confidence alignments
    async fn auto_validate_alignments(
        &self,
        alignments: Vec<SentenceAlignment>
    ) -> Result<Vec<SentenceAlignment>> {
        let mut validated_alignments = Vec::new();

        for mut alignment in alignments {
            if alignment.alignment_confidence >= self.config.auto_validation_threshold {
                alignment.validation_status = ValidationStatus::Validated;
                alignment.last_validated = Some(Instant::now());
            }

            validated_alignments.push(alignment);
        }

        Ok(validated_alignments)
    }

    /// Calculate alignment quality indicators for real-time feedback
    pub async fn calculate_quality_indicators(
        &self,
        alignments: &[SentenceAlignment],
    ) -> Result<AlignmentQualityIndicator> {
        let total_alignments = alignments.len() as f64;
        
        if total_alignments == 0.0 {
            return Ok(AlignmentQualityIndicator {
                overall_quality: 0.0,
                position_consistency: 0.0,
                length_ratio_consistency: 0.0,
                structural_coherence: 0.0,
                user_validation_rate: 0.0,
                problem_areas: Vec::new(),
            });
        }

        // Calculate position consistency
        let position_consistency = self.calculate_position_consistency(alignments);
        
        // Calculate length ratio consistency  
        let length_ratio_consistency = self.calculate_length_ratio_consistency(alignments);
        
        // Calculate structural coherence
        let structural_coherence = self.calculate_structural_coherence(alignments);
        
        // Calculate user validation rate
        let validated_count = alignments.iter()
            .filter(|a| a.validation_status == ValidationStatus::Validated)
            .count() as f64;
        let user_validation_rate = validated_count / total_alignments;

        // Calculate overall quality
        let overall_quality = (
            position_consistency * 0.3 +
            length_ratio_consistency * 0.25 +
            structural_coherence * 0.25 +
            user_validation_rate * 0.2
        );

        // Identify problem areas
        let problem_areas = self.identify_problem_areas(alignments);

        Ok(AlignmentQualityIndicator {
            overall_quality,
            position_consistency,
            length_ratio_consistency,
            structural_coherence,
            user_validation_rate,
            problem_areas,
        })
    }

    /// Learn from user corrections to improve future alignments
    pub async fn learn_from_correction(
        &self,
        original_alignment: SentenceAlignment,
        corrected_alignment: SentenceAlignment,
        correction_reason: String,
    ) -> Result<()> {
        let correction = AlignmentCorrection {
            original_alignment,
            corrected_alignment,
            correction_reason,
            timestamp: Instant::now(),
        };

        let mut ml_model = self.ml_model.write().unwrap();
        
        // Add to correction history
        ml_model.correction_history.push_back(correction.clone());
        
        // Keep only recent corrections (last 1000)
        if ml_model.correction_history.len() > 1000 {
            ml_model.correction_history.pop_front();
        }

        // Update model weights based on the correction
        self.update_ml_weights(&mut ml_model, &correction);

        Ok(())
    }

    /// Synchronize sentence boundaries across multiple panes in real-time
    pub async fn synchronize_sentence_boundaries(
        &self,
        pane_contents: &HashMap<String, String>, // language -> content
        cursor_position: usize,
        source_language: &str,
    ) -> Result<HashMap<String, usize>> {
        let mut synchronized_positions = HashMap::new();

        // Find the sentence containing the cursor in the source language
        let source_content = pane_contents.get(source_language)
            .ok_or_else(|| crate::TradocumentError::Validation("Source language not found".to_string()))?;

        let source_sentences = self.detect_sentence_boundaries(source_content, source_language).await?;
        
        let current_sentence_index = source_sentences.iter()
            .position(|s| cursor_position >= s.start_offset && cursor_position <= s.end_offset);

        if let Some(sentence_index) = current_sentence_index {
            // Find corresponding sentences in other languages
            for (language, content) in pane_contents {
                if language == source_language {
                    synchronized_positions.insert(language.clone(), cursor_position);
                    continue;
                }

                let target_sentences = self.detect_sentence_boundaries(content, language).await?;
                
                // Get alignment between source and target language
                let alignments = self.align_sentences(
                    source_content, 
                    content, 
                    source_language, 
                    language
                ).await?;

                // Find the alignment for the current sentence
                if let Some(alignment) = alignments.iter()
                    .find(|a| a.source_sentence.start_offset <= cursor_position && 
                              a.source_sentence.end_offset >= cursor_position) {
                    synchronized_positions.insert(
                        language.clone(), 
                        alignment.target_sentence.start_offset
                    );
                } else if sentence_index < target_sentences.len() {
                    // Fallback to position-based synchronization
                    synchronized_positions.insert(
                        language.clone(), 
                        target_sentences[sentence_index].start_offset
                    );
                }
            }
        }

        Ok(synchronized_positions)
    }

    /// Get alignment statistics for reporting
    pub async fn get_alignment_statistics(
        &self,
        language_pair: (String, String),
    ) -> Result<Option<AlignmentStatistics>> {
        let stats = self.statistics.read().unwrap();
        let key = format!("{}:{}", language_pair.0, language_pair.1);
        Ok(stats.get(&key).cloned())
    }

    // Helper methods

    fn detect_boundary_type(&self, sentence_text: &str) -> BoundaryType {
        let text = sentence_text.trim();
        if text.ends_with('?') {
            BoundaryType::Question
        } else if text.ends_with('!') {
            BoundaryType::Exclamation
        } else if text.ends_with("...") || text.ends_with("…") {
            BoundaryType::Ellipsis
        } else if text.ends_with('.') {
            BoundaryType::Period
        } else {
            BoundaryType::EndOfParagraph
        }
    }

    fn calculate_boundary_confidence(
        &self,
        sentence_text: &str,
        profile: &LanguageProfile,
        boundary_type: &BoundaryType,
    ) -> f64 {
        let length = sentence_text.len() as f64;
        let word_count = sentence_text.split_whitespace().count() as f64;

        // Base confidence from boundary type
        let type_confidence = match boundary_type {
            BoundaryType::Period => 0.8,
            BoundaryType::Question => 0.9,
            BoundaryType::Exclamation => 0.9,
            BoundaryType::Ellipsis => 0.7,
            BoundaryType::EndOfParagraph => 0.9,
            BoundaryType::Custom(_) => 0.6,
        };

        // Length-based confidence
        let length_deviation = (length - profile.average_sentence_length).abs() / profile.length_variance;
        let length_confidence = 1.0 - (length_deviation / 3.0).min(1.0);

        // Word count confidence
        let word_deviation = (word_count - profile.typical_word_count).abs() / (profile.typical_word_count * 0.5);
        let word_confidence = 1.0 - (word_deviation / 2.0).min(1.0);

        // Combined confidence
        (type_confidence * 0.5 + length_confidence * 0.3 + word_confidence * 0.2).min(1.0).max(0.1)
    }

    async fn calculate_alignment_confidence(
        &self,
        source_sentence: &SentenceBoundary,
        target_sentence: &SentenceBoundary,
        source_position: f64,
        target_position: f64,
        source_language: &str,
        target_language: &str,
    ) -> Result<f64> {
        // Position similarity
        let position_similarity = 1.0 - (source_position - target_position).abs();
        
        // Length ratio similarity
        let profiles = self.language_profiles.read().unwrap();
        let default_source_profile = LanguageProfile::english();
        let default_target_profile = LanguageProfile::english();
        let source_profile = profiles.get(source_language).unwrap_or(&default_source_profile);
        let target_profile = profiles.get(target_language).unwrap_or(&default_target_profile);
        
        let expected_ratio = target_profile.average_sentence_length / source_profile.average_sentence_length;
        let actual_ratio = target_sentence.text.len() as f64 / source_sentence.text.len().max(1) as f64;
        let ratio_similarity = 1.0 - ((actual_ratio / expected_ratio - 1.0).abs() / 2.0).min(1.0);

        // Structure similarity (simplified)
        let structure_similarity = self.calculate_structure_similarity(
            &source_sentence.text, 
            &target_sentence.text
        );

        // Weighted combination
        let confidence = position_similarity * self.config.position_weight +
                        ratio_similarity * self.config.length_weight +
                        structure_similarity * self.config.structure_weight;

        Ok(confidence.min(1.0).max(0.0))
    }

    fn calculate_structure_similarity(&self, source_text: &str, target_text: &str) -> f64 {
        // Simplified structure similarity based on punctuation patterns
        let source_punct: Vec<char> = source_text.chars()
            .filter(|c| c.is_ascii_punctuation())
            .collect();
        let target_punct: Vec<char> = target_text.chars()
            .filter(|c| c.is_ascii_punctuation())
            .collect();

        if source_punct.is_empty() && target_punct.is_empty() {
            return 1.0;
        }

        let common_count = source_punct.iter()
            .filter(|&c| target_punct.contains(c))
            .count();
        
        let total_unique = source_punct.len().max(target_punct.len());
        
        if total_unique == 0 {
            1.0
        } else {
            common_count as f64 / total_unique as f64
        }
    }

    fn extract_alignment_features(&self, alignment: &SentenceAlignment) -> HashMap<String, f64> {
        let mut features = HashMap::new();
        
        let source_len = alignment.source_sentence.text.len() as f64;
        let target_len = alignment.target_sentence.text.len() as f64;
        
        features.insert("length_ratio".to_string(), target_len / source_len.max(1.0));
        features.insert("source_length".to_string(), source_len);
        features.insert("target_length".to_string(), target_len);
        features.insert("position_similarity".to_string(), 
            1.0 - (alignment.source_sentence.start_offset as f64 - 
                   alignment.target_sentence.start_offset as f64).abs() / 1000.0);
        features.insert("structure_similarity".to_string(), 
            self.calculate_structure_similarity(
                &alignment.source_sentence.text, 
                &alignment.target_sentence.text
            ));
        
        features
    }

    fn calculate_ml_confidence(
        &self,
        features: &HashMap<String, f64>,
        ml_model: &AlignmentMLModel,
    ) -> f64 {
        let mut confidence_adjustment = 0.0;
        
        for (feature_name, feature_value) in features {
            if let Some(&weight) = ml_model.feature_weights.get(feature_name) {
                confidence_adjustment += feature_value * weight;
            }
        }
        
        confidence_adjustment.tanh() // Normalize to [-1, 1]
    }

    fn update_ml_weights(&self, ml_model: &mut AlignmentMLModel, correction: &AlignmentCorrection) {
        // Simple gradient descent update
        let original_features = self.extract_alignment_features(&correction.original_alignment);
        let corrected_features = self.extract_alignment_features(&correction.corrected_alignment);
        
        for feature_name in original_features.keys() {
            if let (Some(&original_value), Some(&corrected_value)) = 
                (original_features.get(feature_name), corrected_features.get(feature_name)) {
                
                let error = corrected_value - original_value;
                if let Some(weight) = ml_model.feature_weights.get_mut(feature_name) {
                    *weight += ml_model.learning_rate * error;
                    *weight = weight.clamp(-2.0, 2.0); // Prevent extreme weights
                }
            }
        }
    }

    fn calculate_position_consistency(&self, alignments: &[SentenceAlignment]) -> f64 {
        if alignments.len() < 2 {
            return 1.0;
        }

        let mut consistency_scores = Vec::new();
        
        for i in 1..alignments.len() {
            let prev = &alignments[i - 1];
            let curr = &alignments[i];
            
            let source_order = curr.source_sentence.start_offset > prev.source_sentence.start_offset;
            let target_order = curr.target_sentence.start_offset > prev.target_sentence.start_offset;
            
            consistency_scores.push(if source_order == target_order { 1.0 } else { 0.0 });
        }

        consistency_scores.iter().sum::<f64>() / consistency_scores.len() as f64
    }

    fn calculate_length_ratio_consistency(&self, alignments: &[SentenceAlignment]) -> f64 {
        if alignments.is_empty() {
            return 1.0;
        }

        let ratios: Vec<f64> = alignments.iter()
            .map(|a| {
                let source_len = a.source_sentence.text.len().max(1) as f64;
                let target_len = a.target_sentence.text.len() as f64;
                target_len / source_len
            })
            .collect();

        if ratios.len() < 2 {
            return 1.0;
        }

        let mean_ratio = ratios.iter().sum::<f64>() / ratios.len() as f64;
        let variance = ratios.iter()
            .map(|r| (r - mean_ratio).powi(2))
            .sum::<f64>() / ratios.len() as f64;
        
        // Convert variance to consistency score (lower variance = higher consistency)
        1.0 - variance.sqrt().min(1.0)
    }

    fn calculate_structural_coherence(&self, alignments: &[SentenceAlignment]) -> f64 {
        if alignments.is_empty() {
            return 1.0;
        }

        let coherence_scores: Vec<f64> = alignments.iter()
            .map(|a| self.calculate_structure_similarity(
                &a.source_sentence.text, 
                &a.target_sentence.text
            ))
            .collect();

        coherence_scores.iter().sum::<f64>() / coherence_scores.len() as f64
    }

    fn identify_problem_areas(&self, alignments: &[SentenceAlignment]) -> Vec<ProblemArea> {
        let mut problem_areas = Vec::new();

        for (i, alignment) in alignments.iter().enumerate() {
            let mut issues = Vec::new();

            // Check for length mismatches
            let source_len = alignment.source_sentence.text.len() as f64;
            let target_len = alignment.target_sentence.text.len() as f64;
            let ratio = target_len / source_len.max(1.0);
            
            if ratio > 3.0 || ratio < 0.3 {
                issues.push((AlignmentIssue::LengthMismatch, 0.8));
            }

            // Check for low confidence
            if alignment.alignment_confidence < 0.5 {
                issues.push((AlignmentIssue::BoundaryDetectionError, 0.6));
            }

            // Check for structural divergence
            let structure_sim = self.calculate_structure_similarity(
                &alignment.source_sentence.text, 
                &alignment.target_sentence.text
            );
            if structure_sim < 0.3 {
                issues.push((AlignmentIssue::StructuralDivergence, 0.7));
            }

            // Create problem areas for significant issues
            for (issue_type, severity) in issues {
                if severity > 0.5 {
                    problem_areas.push(ProblemArea {
                        start_position: alignment.source_sentence.start_offset,
                        end_position: alignment.source_sentence.end_offset,
                        issue_type: issue_type.clone(),
                        severity,
                        suggestion: self.generate_suggestion(&issue_type),
                    });
                }
            }
        }

        problem_areas
    }

    fn generate_suggestion(&self, issue_type: &AlignmentIssue) -> String {
        match issue_type {
            AlignmentIssue::LengthMismatch => 
                "Consider splitting or merging sentences to better match the translation structure.".to_string(),
            AlignmentIssue::StructuralDivergence => 
                "Review sentence structure - the translation may have different punctuation or formatting.".to_string(),
            AlignmentIssue::MissingSentence => 
                "A sentence appears to be missing in the translation.".to_string(),
            AlignmentIssue::ExtraSentence => 
                "An extra sentence appears in the translation.".to_string(),
            AlignmentIssue::OrderMismatch => 
                "Sentence order differs between source and translation.".to_string(),
            AlignmentIssue::BoundaryDetectionError => 
                "Sentence boundary detection may be incorrect - check punctuation.".to_string(),
        }
    }

    async fn update_statistics(
        &self,
        alignments: &[SentenceAlignment],
        source_language: &str,
        target_language: &str,
        processing_time: Duration,
    ) -> Result<()> {
        let key = format!("{}:{}", source_language, target_language);
        let mut stats = self.statistics.write().unwrap();
        
        let total_sentences = alignments.len();
        let aligned_sentences = alignments.iter()
            .filter(|a| a.alignment_confidence >= self.config.confidence_threshold)
            .count();
        let validated_alignments = alignments.iter()
            .filter(|a| a.validation_status == ValidationStatus::Validated)
            .count();
        
        let average_confidence = if total_sentences > 0 {
            alignments.iter().map(|a| a.alignment_confidence).sum::<f64>() / total_sentences as f64
        } else {
            0.0
        };

        let alignment_accuracy = if total_sentences > 0 {
            aligned_sentences as f64 / total_sentences as f64
        } else {
            0.0
        };

        let statistics = AlignmentStatistics {
            total_sentences,
            aligned_sentences,
            validated_alignments,
            average_confidence,
            alignment_accuracy,
            processing_time_ms: processing_time.as_millis() as u64,
            language_pair: (source_language.to_string(), target_language.to_string()),
        };

        stats.insert(key, statistics);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sentence_boundary_detection() {
        let service = SentenceAlignmentService::new(AlignmentConfig::default());
        let text = "This is the first sentence. This is the second! Is this the third?";
        
        let boundaries = service.detect_sentence_boundaries(text, "en").await.unwrap();
        assert_eq!(boundaries.len(), 3);
        assert_eq!(boundaries[0].boundary_type, BoundaryType::Period);
        assert_eq!(boundaries[1].boundary_type, BoundaryType::Exclamation);
        assert_eq!(boundaries[2].boundary_type, BoundaryType::Question);
    }

    #[tokio::test]
    async fn test_sentence_alignment() {
        let service = SentenceAlignmentService::new(AlignmentConfig::default());
        let source = "Hello world. How are you?";
        let target = "Hola mundo. ¿Cómo estás?";
        
        let alignments = service.align_sentences(source, target, "en", "es").await.unwrap();
        assert_eq!(alignments.len(), 2);
        assert!(alignments[0].alignment_confidence > 0.0);
    }

    #[tokio::test]
    async fn test_quality_indicators() {
        let service = SentenceAlignmentService::new(AlignmentConfig::default());
        let source = "First sentence. Second sentence.";
        let target = "Primera oración. Segunda oración.";
        
        let alignments = service.align_sentences(source, target, "en", "es").await.unwrap();
        let quality = service.calculate_quality_indicators(&alignments).await.unwrap();
        
        assert!(quality.overall_quality >= 0.0);
        assert!(quality.overall_quality <= 1.0);
    }

    #[tokio::test]
    async fn test_synchronization() {
        let service = SentenceAlignmentService::new(AlignmentConfig::default());
        let mut pane_contents = HashMap::new();
        pane_contents.insert("en".to_string(), "First sentence. Second sentence.".to_string());
        pane_contents.insert("es".to_string(), "Primera oración. Segunda oración.".to_string());
        
        let sync_positions = service.synchronize_sentence_boundaries(
            &pane_contents, 
            20, // Cursor in second sentence
            "en"
        ).await.unwrap();
        
        assert!(sync_positions.contains_key("en"));
        assert!(sync_positions.contains_key("es"));
    }
}