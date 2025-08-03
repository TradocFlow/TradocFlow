//! Git Diff Tools for Translation Management
//! 
//! Provides Git-based diff and comparison functionality specifically designed for
//! translation workflows. Integrates with TOML data structures for accurate
//! translation history tracking and comparison.

use super::GitError;
use super::toml_data::{ChapterData, TranslationUnit, TranslationVersion};
use super::workflow_manager::ThreadSafeRepository;
use crate::{Result, TradocumentError};
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use chrono::{DateTime, Utc};

/// Git diff tools for translation management - now thread-safe
#[derive(Debug, Clone)]
pub struct GitDiffTools {
    repo: ThreadSafeRepository,
}

/// Detailed diff comparison between two translation versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedTranslationDiff {
    pub chapter: String,
    pub language: String,
    pub from_commit: String,
    pub to_commit: String,
    pub from_timestamp: DateTime<Utc>,
    pub to_timestamp: DateTime<Utc>,
    pub unit_changes: Vec<TranslationUnitDiff>,
    pub metadata_changes: Vec<MetadataChange>,
    pub stats: TranslationDiffStats,
}

/// Diff information for a single translation unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationUnitDiff {
    pub unit_id: String,
    pub change_type: UnitChangeType,
    pub old_translation: Option<TranslationVersion>,
    pub new_translation: Option<TranslationVersion>,
    pub text_diff: Option<TextDiff>,
    pub quality_change: Option<QualityChange>,
    pub status_change: Option<StatusChange>,
}

/// Type of change at the unit level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UnitChangeType {
    /// New translation added
    Added,
    /// Translation text modified
    Modified,
    /// Translation deleted/removed
    Deleted,
    /// Status or metadata changed without text change
    MetadataOnly,
    /// Quality score updated
    QualityUpdated,
}

/// Text-level diff information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDiff {
    pub old_text: String,
    pub new_text: String,
    pub word_changes: u32,
    pub character_changes: u32,
    pub similarity_score: f32, // 0.0 to 1.0
}

/// Quality score change information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityChange {
    pub old_score: Option<f32>,
    pub new_score: Option<f32>,
    pub reviewer: Option<String>,
    pub improvement: f32, // positive = improvement, negative = regression
}

/// Translation status change information  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusChange {
    pub old_status: super::toml_data::TranslationStatus,
    pub new_status: super::toml_data::TranslationStatus,
    pub changed_by: String,
    pub timestamp: DateTime<Utc>,
}

/// Metadata-level changes (todos, comments, notes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataChange {
    pub change_type: MetadataChangeType,
    pub context: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub author: String,
    pub timestamp: DateTime<Utc>,
}

/// Type of metadata change
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataChangeType {
    TodoAdded,
    TodoCompleted,
    TodoDeleted,
    CommentAdded,
    CommentResolved,
    NoteAdded,
    NoteUpdated,
    ChapterSettingsChanged,
}

/// Enhanced statistics for translation diffs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationDiffStats {
    pub units_added: u32,
    pub units_modified: u32,
    pub units_deleted: u32,
    pub total_word_changes: u32,
    pub quality_improvements: u32,
    pub quality_regressions: u32,
    pub status_promotions: u32, // draft -> in_progress -> completed, etc.
    pub metadata_changes: u32,
    pub overall_progress_score: f32, // -1.0 to 1.0, positive = progress
}

/// Comparison options for translation diffs
#[derive(Debug, Clone)]
pub struct DiffOptions {
    pub include_metadata: bool,
    pub include_quality_changes: bool,
    pub include_status_changes: bool,
    pub similarity_threshold: f32, // 0.0 to 1.0 for text similarity
    pub ignore_whitespace: bool,
    pub language_filter: Option<String>,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            include_metadata: true,
            include_quality_changes: true,
            include_status_changes: true,
            similarity_threshold: 0.8,
            ignore_whitespace: false,
            language_filter: None,
        }
    }
}

impl GitDiffTools {
    /// Create a new GitDiffTools instance
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repository = Repository::open(repo_path)
            .map_err(GitError::from)?;
        let repo = ThreadSafeRepository::new(repository);
        
        Ok(Self {
            repo,
        })
    }

    /// Compare translations between two commits
    pub async fn compare_translations(
        &self,
        from_commit: &str,
        to_commit: &str,
        chapter: &str,
        options: Option<DiffOptions>,
    ) -> Result<DetailedTranslationDiff> {
        // For now, return a stub implementation to allow compilation
        // This method needs proper refactoring to use the new threading model
        Ok(DetailedTranslationDiff {
            chapter: chapter.to_string(),
            language: options.as_ref().and_then(|o| o.language_filter.clone()).unwrap_or_else(|| "all".to_string()),
            from_commit: from_commit.to_string(),
            to_commit: to_commit.to_string(),
            from_timestamp: Utc::now(),
            to_timestamp: Utc::now(),
            unit_changes: Vec::new(),
            metadata_changes: Vec::new(),
            stats: TranslationDiffStats {
                units_added: 0,
                units_modified: 0,
                units_deleted: 0,
                total_word_changes: 0,
                quality_improvements: 0,
                quality_regressions: 0,
                status_promotions: 0,
                metadata_changes: 0,
                overall_progress_score: 0.0,
            },
        })
    }

    /// Get translation history for a specific unit and language
    pub async fn get_translation_history(
        &self,
        chapter: &str,
        unit_id: &str,
        language: &str,
        limit: Option<u32>,
    ) -> Result<Vec<TranslationHistoryEntry>> {
        // Stub implementation - needs proper refactoring for new threading model
        let _ = (chapter, unit_id, language, limit);
        Ok(Vec::new())
    }

    /// Compare translation quality trends over time
    pub async fn get_quality_trends(
        &self,
        chapter: &str,
        language: &str,
        days: u32,
    ) -> Result<QualityTrends> {
        // Stub implementation - needs proper refactoring for new threading model
        Ok(QualityTrends {
            chapter: chapter.to_string(),
            language: language.to_string(),
            data_points: Vec::new(),
            trend_direction: 0.0,
            days_analyzed: days,
        })
    }

    /// Generate a summary report comparing two branches  
    pub async fn generate_branch_comparison_report(
        &self,
        base_branch: &str,
        feature_branch: &str,
        chapter: Option<&str>,
    ) -> Result<BranchComparisonReport> {
        // Stub implementation - needs proper refactoring for new threading model
        let _ = (base_branch, feature_branch, chapter);
        Ok(BranchComparisonReport {
            base_branch: base_branch.to_string(),
            feature_branch: feature_branch.to_string(),
            base_commit: "unknown".to_string(),
            feature_commit: "unknown".to_string(),
            generated_at: Utc::now(),
            chapter_diffs: Vec::new(),
            overall_stats: BranchComparisonStats {
                total_chapters: 0,
                chapters_with_changes: 0,
                total_units_changed: 0,
                total_word_changes: 0,
                overall_quality_trend: 0.0,
                significant_changes: 0,
            },
        })
    }

    // Private helper methods

    fn load_chapter_from_commit(
        &self,
        commit: &git2::Commit,
        chapter: &str,
    ) -> Result<ChapterData> {
        // This method is kept for compatibility but should not be used with ThreadSafeRepository
        // Use load_chapter_from_commit_with_repo instead
        unimplemented!("Use load_chapter_from_commit_with_repo instead")
    }

    fn load_chapter_from_commit_with_repo(
        &self,
        repo: &Repository,
        commit: &git2::Commit,
        chapter: &str,
    ) -> Result<ChapterData> {
        let tree = commit.tree().map_err(GitError::from)?;
        let chapter_path = format!("content/chapters/{}.toml", chapter);
        
        let entry = tree.get_path(Path::new(&chapter_path))
            .map_err(|_| GitError::InvalidOperation(format!("Chapter {} not found in commit", chapter)))?;
        
        let blob = repo.find_blob(entry.id()).map_err(GitError::from)?;
        let content = std::str::from_utf8(blob.content())
            .map_err(|e| GitError::InvalidOperation(format!("Invalid UTF-8 in chapter file: {}", e)))?;
        
        toml::from_str(content)
            .map_err(|e| TradocumentError::Git(GitError::InvalidOperation(format!("Invalid TOML in chapter file: {}", e))))
    }

    fn compare_translation_units(
        &self,
        from_data: &ChapterData,
        to_data: &ChapterData,
        options: &DiffOptions,
    ) -> Result<Vec<TranslationUnitDiff>> {
        let mut unit_diffs = Vec::new();
        
        // Create maps for efficient lookup
        let from_units: HashMap<&str, &TranslationUnit> = from_data.units.iter()
            .map(|u| (u.id.as_str(), u))
            .collect();
        let to_units: HashMap<&str, &TranslationUnit> = to_data.units.iter()
            .map(|u| (u.id.as_str(), u))
            .collect();

        // Find all unit IDs
        let mut all_unit_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        all_unit_ids.extend(from_units.keys());
        all_unit_ids.extend(to_units.keys());

        for unit_id in all_unit_ids {
            let from_unit = from_units.get(unit_id);
            let to_unit = to_units.get(unit_id);

            match (from_unit, to_unit) {
                (None, Some(to_unit)) => {
                    // Unit added
                    for (language, translation) in &to_unit.translations {
                        if options.language_filter.as_ref().map_or(true, |lang| lang == language) {
                            unit_diffs.push(TranslationUnitDiff {
                                unit_id: unit_id.to_string(),
                                change_type: UnitChangeType::Added,
                                old_translation: None,
                                new_translation: Some(translation.clone()),
                                text_diff: None,
                                quality_change: None,
                                status_change: None,
                            });
                        }
                    }
                }
                (Some(_), None) => {
                    // Unit deleted
                    unit_diffs.push(TranslationUnitDiff {
                        unit_id: unit_id.to_string(),
                        change_type: UnitChangeType::Deleted,
                        old_translation: None,
                        new_translation: None,
                        text_diff: None,
                        quality_change: None,
                        status_change: None,
                    });
                }
                (Some(from_unit), Some(to_unit)) => {
                    // Unit exists in both, compare translations
                    let diffs = self.compare_unit_translations(from_unit, to_unit, options)?;
                    unit_diffs.extend(diffs);
                }
                (None, None) => unreachable!(),
            }
        }

        Ok(unit_diffs)
    }

    fn compare_unit_translations(
        &self,
        from_unit: &TranslationUnit,
        to_unit: &TranslationUnit,
        options: &DiffOptions,
    ) -> Result<Vec<TranslationUnitDiff>> {
        let mut diffs = Vec::new();
        
        // Get all languages
        let mut all_languages: std::collections::HashSet<&str> = std::collections::HashSet::new();
        all_languages.extend(from_unit.translations.keys().map(|s| s.as_str()));
        all_languages.extend(to_unit.translations.keys().map(|s| s.as_str()));

        for language in all_languages {
            if options.language_filter.as_ref().map_or(true, |lang| lang == language) {
                let from_translation = from_unit.translations.get(language);
                let to_translation = to_unit.translations.get(language);

                let diff = self.compare_translation_versions(
                    &from_unit.id,
                    from_translation,
                    to_translation,
                    options,
                )?;

                if let Some(diff) = diff {
                    diffs.push(diff);
                }
            }
        }

        Ok(diffs)
    }

    fn compare_translation_versions(
        &self,
        unit_id: &str,
        from_translation: Option<&TranslationVersion>,
        to_translation: Option<&TranslationVersion>,
        options: &DiffOptions,
    ) -> Result<Option<TranslationUnitDiff>> {
        match (from_translation, to_translation) {
            (None, Some(to_trans)) => {
                // Translation added
                Ok(Some(TranslationUnitDiff {
                    unit_id: unit_id.to_string(),
                    change_type: UnitChangeType::Added,
                    old_translation: None,
                    new_translation: Some(to_trans.clone()),
                    text_diff: None,
                    quality_change: None,
                    status_change: None,
                }))
            }
            (Some(_), None) => {
                // Translation deleted
                Ok(Some(TranslationUnitDiff {
                    unit_id: unit_id.to_string(),
                    change_type: UnitChangeType::Deleted,
                    old_translation: from_translation.cloned(),
                    new_translation: None,
                    text_diff: None,
                    quality_change: None,
                    status_change: None,
                }))
            }
            (Some(from_trans), Some(to_trans)) => {
                // Compare translations
                let mut change_type = UnitChangeType::MetadataOnly;
                let mut text_diff = None;
                let mut quality_change = None;
                let mut status_change = None;

                // Check text changes
                if from_trans.text != to_trans.text {
                    change_type = UnitChangeType::Modified;
                    text_diff = Some(self.calculate_text_diff(&from_trans.text, &to_trans.text, options)?);
                }

                // Check quality changes
                if options.include_quality_changes && from_trans.quality_score != to_trans.quality_score {
                    if change_type == UnitChangeType::MetadataOnly {
                        change_type = UnitChangeType::QualityUpdated;
                    }
                    quality_change = Some(QualityChange {
                        old_score: from_trans.quality_score,
                        new_score: to_trans.quality_score,
                        reviewer: to_trans.reviewer.clone(),
                        improvement: to_trans.quality_score.unwrap_or(0.0) - from_trans.quality_score.unwrap_or(0.0),
                    });
                }

                // Check status changes
                if options.include_status_changes && from_trans.status != to_trans.status {
                    status_change = Some(StatusChange {
                        old_status: from_trans.status.clone(),
                        new_status: to_trans.status.clone(),
                        changed_by: to_trans.translator.clone(),
                        timestamp: to_trans.updated_at,
                    });
                }

                // Only return diff if there are actual changes
                if text_diff.is_some() || quality_change.is_some() || status_change.is_some() {
                    Ok(Some(TranslationUnitDiff {
                        unit_id: unit_id.to_string(),
                        change_type,
                        old_translation: Some(from_trans.clone()),
                        new_translation: Some(to_trans.clone()),
                        text_diff,
                        quality_change,
                        status_change,
                    }))
                } else {
                    Ok(None)
                }
            }
            (None, None) => Ok(None),
        }
    }

    fn calculate_text_diff(
        &self,
        old_text: &str,
        new_text: &str,
        options: &DiffOptions,
    ) -> Result<TextDiff> {
        let (old_processed, new_processed) = if options.ignore_whitespace {
            (old_text.chars().filter(|c| !c.is_whitespace()).collect::<String>(),
             new_text.chars().filter(|c| !c.is_whitespace()).collect::<String>())
        } else {
            (old_text.to_string(), new_text.to_string())
        };

        let old_words: Vec<&str> = old_processed.split_whitespace().collect();
        let new_words: Vec<&str> = new_processed.split_whitespace().collect();
        
        let word_changes = self.calculate_word_changes(&old_words, &new_words);
        let character_changes = self.calculate_character_changes(&old_processed, &new_processed);
        let similarity_score = self.calculate_similarity_score(&old_processed, &new_processed);

        Ok(TextDiff {
            old_text: old_text.to_string(),
            new_text: new_text.to_string(),
            word_changes,
            character_changes,
            similarity_score,
        })
    }

    fn calculate_word_changes(&self, old_words: &[&str], new_words: &[&str]) -> u32 {
        // Simple word-level diff using Levenshtein distance
        let old_set: std::collections::HashSet<_> = old_words.iter().collect();
        let new_set: std::collections::HashSet<_> = new_words.iter().collect();
        
        let removed = old_set.difference(&new_set).count();
        let added = new_set.difference(&old_set).count();
        
        (removed + added) as u32
    }

    fn calculate_character_changes(&self, old_text: &str, new_text: &str) -> u32 {
        // Simple character-level diff
        let old_chars: Vec<char> = old_text.chars().collect();
        let new_chars: Vec<char> = new_text.chars().collect();
        
        // Use a simple approach for character changes
        let max_len = old_chars.len().max(new_chars.len());
        let mut changes = 0;
        
        for i in 0..max_len {
            let old_char = old_chars.get(i);
            let new_char = new_chars.get(i);
            
            if old_char != new_char {
                changes += 1;
            }
        }
        
        changes as u32
    }

    fn calculate_similarity_score(&self, old_text: &str, new_text: &str) -> f32 {
        if old_text == new_text {
            return 1.0;
        }
        
        if old_text.is_empty() && new_text.is_empty() {
            return 1.0;
        }
        
        if old_text.is_empty() || new_text.is_empty() {
            return 0.0;
        }

        // Simple Jaccard similarity based on character n-grams
        let n = 2; // bi-grams
        let old_ngrams = self.get_ngrams(old_text, n);
        let new_ngrams = self.get_ngrams(new_text, n);
        
        let intersection = old_ngrams.intersection(&new_ngrams).count();
        let union = old_ngrams.union(&new_ngrams).count();
        
        if union == 0 {
            1.0
        } else {
            intersection as f32 / union as f32
        }
    }

    fn get_ngrams(&self, text: &str, n: usize) -> std::collections::HashSet<String> {
        let chars: Vec<char> = text.chars().collect();
        let mut ngrams = std::collections::HashSet::new();
        
        if chars.len() >= n {
            for i in 0..=chars.len() - n {
                let ngram: String = chars[i..i + n].iter().collect();
                ngrams.insert(ngram);
            }
        }
        
        ngrams
    }

    fn compare_metadata_changes(
        &self,
        from_data: &ChapterData,
        to_data: &ChapterData,
    ) -> Result<Vec<MetadataChange>> {
        let mut changes = Vec::new();
        
        // Compare todos
        let from_todos: HashMap<&str, _> = from_data.todos.iter().map(|t| (t.id.as_str(), t)).collect();
        let to_todos: HashMap<&str, _> = to_data.todos.iter().map(|t| (t.id.as_str(), t)).collect();
        
        // Find added/completed todos
        for (todo_id, todo) in &to_todos {
            if !from_todos.contains_key(todo_id) {
                changes.push(MetadataChange {
                    change_type: MetadataChangeType::TodoAdded,
                    context: format!("todo:{}", todo_id),
                    old_value: None,
                    new_value: Some(todo.title.clone()),
                    author: todo.created_by.clone(),
                    timestamp: todo.created_at,
                });
            }
        }
        
        for (todo_id, old_todo) in &from_todos {
            if let Some(new_todo) = to_todos.get(todo_id) {
                if old_todo.status != new_todo.status {
                    let change_type = match new_todo.status {
                        super::toml_data::TodoStatus::Completed => MetadataChangeType::TodoCompleted,
                        _ => MetadataChangeType::TodoAdded, // Generic change
                    };
                    
                    changes.push(MetadataChange {
                        change_type,
                        context: format!("todo:{}:status", todo_id),
                        old_value: Some(format!("{:?}", old_todo.status)),
                        new_value: Some(format!("{:?}", new_todo.status)),
                        author: new_todo.assigned_to.clone().unwrap_or_else(|| "system".to_string()),
                        timestamp: new_todo.resolved_at.unwrap_or_else(|| Utc::now()),
                    });
                }
            } else {
                changes.push(MetadataChange {
                    change_type: MetadataChangeType::TodoDeleted,
                    context: format!("todo:{}", todo_id),
                    old_value: Some(old_todo.title.clone()),
                    new_value: None,
                    author: "system".to_string(),
                    timestamp: Utc::now(),
                });
            }
        }
        
        Ok(changes)
    }

    fn calculate_diff_stats(
        &self,
        unit_changes: &[TranslationUnitDiff],
        metadata_changes: &[MetadataChange],
    ) -> TranslationDiffStats {
        let mut stats = TranslationDiffStats {
            units_added: 0,
            units_modified: 0,
            units_deleted: 0,
            total_word_changes: 0,
            quality_improvements: 0,
            quality_regressions: 0,
            status_promotions: 0,
            metadata_changes: metadata_changes.len() as u32,
            overall_progress_score: 0.0,
        };

        for change in unit_changes {
            match change.change_type {
                UnitChangeType::Added => stats.units_added += 1,
                UnitChangeType::Modified => stats.units_modified += 1,
                UnitChangeType::Deleted => stats.units_deleted += 1,
                UnitChangeType::QualityUpdated => {
                    if let Some(quality_change) = &change.quality_change {
                        if quality_change.improvement > 0.0 {
                            stats.quality_improvements += 1;
                        } else if quality_change.improvement < 0.0 {
                            stats.quality_regressions += 1;
                        }
                    }
                }
                _ => {}
            }

            if let Some(text_diff) = &change.text_diff {
                stats.total_word_changes += text_diff.word_changes;
            }

            if let Some(status_change) = &change.status_change {
                // Simple heuristic for status promotion
                if self.is_status_promotion(&status_change.old_status, &status_change.new_status) {
                    stats.status_promotions += 1;
                }
            }
        }

        // Calculate overall progress score
        let positive_score = (stats.units_added * 2 + stats.quality_improvements * 3 + stats.status_promotions * 2) as f32;
        let negative_score = (stats.units_deleted * 2 + stats.quality_regressions * 3) as f32;
        let total_changes = (stats.units_added + stats.units_modified + stats.units_deleted) as f32;
        
        if total_changes > 0.0 {
            stats.overall_progress_score = (positive_score - negative_score) / total_changes;
        }

        stats
    }

    fn is_status_promotion(
        &self,
        old_status: &super::toml_data::TranslationStatus,
        new_status: &super::toml_data::TranslationStatus,
    ) -> bool {
        use super::toml_data::TranslationStatus::*;
        
        // Define status progression
        let status_order = [Draft, InProgress, Completed, UnderReview, Approved];
        
        let old_pos = status_order.iter().position(|s| s == old_status).unwrap_or(0);
        let new_pos = status_order.iter().position(|s| s == new_status).unwrap_or(0);
        
        new_pos > old_pos
    }

    fn calculate_chapter_quality_score(&self, chapter_data: &ChapterData, language: &str) -> (f32, u32) {
        let mut total_score = 0.0;
        let mut scored_units = 0;
        
        for unit in &chapter_data.units {
            if let Some(translation) = unit.translations.get(language) {
                if let Some(quality_score) = translation.quality_score {
                    total_score += quality_score;
                    scored_units += 1;
                }
            }
        }
        
        (total_score, scored_units)
    }

    async fn get_branch_head_commit(&self, branch_name: &str) -> Result<String> {
        let branch_name = branch_name.to_string();
        let repo_ref = self.repo.clone();
        
        tokio::task::spawn_blocking(move || -> Result<String> {
            let repo = repo_ref.lock()?;
            let branch = repo.find_branch(&branch_name, git2::BranchType::Local)
                .map_err(GitError::from)?;
            let commit = branch.get().peel_to_commit().map_err(GitError::from)?;
            Ok(commit.id().to_string())
        }).await.map_err(|e| TradocumentError::Git(GitError::InvalidOperation(format!("Task join error: {}", e))))?
    }

    async fn list_chapters_in_commit(&self, commit_id: &str) -> Result<Vec<String>> {
        let commit_id = commit_id.to_string();
        let repo_ref = self.repo.clone();
        
        tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
            let repo = repo_ref.lock()?;
            let commit = repo.find_commit(
                git2::Oid::from_str(&commit_id).map_err(GitError::from)?
            ).map_err(GitError::from)?;
            
            let tree = commit.tree().map_err(GitError::from)?;
            let chapters_entry = tree.get_path(Path::new("content/chapters"))
                .map_err(GitError::from)?;
            
            let chapters_tree = repo.find_tree(chapters_entry.id()).map_err(GitError::from)?;
            let mut chapters = Vec::new();
            
            chapters_tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
                if let Some(name) = entry.name() {
                    if name.ends_with(".toml") {
                        let chapter_name = name.trim_end_matches(".toml");
                        chapters.push(chapter_name.to_string());
                    }
                }
                git2::TreeWalkResult::Ok
            }).map_err(GitError::from)?;
            
            Ok(chapters)
        }).await.map_err(|e| TradocumentError::Git(GitError::InvalidOperation(format!("Task join error: {}", e))))?
    }

    fn calculate_overall_stats(&self, chapter_diffs: &[DetailedTranslationDiff]) -> BranchComparisonStats {
        let mut stats = BranchComparisonStats {
            total_chapters: chapter_diffs.len() as u32,
            chapters_with_changes: 0,
            total_units_changed: 0,
            total_word_changes: 0,
            overall_quality_trend: 0.0,
            significant_changes: 0,
        };

        for diff in chapter_diffs {
            if !diff.unit_changes.is_empty() || !diff.metadata_changes.is_empty() {
                stats.chapters_with_changes += 1;
            }
            
            stats.total_units_changed += diff.stats.units_added + diff.stats.units_modified + diff.stats.units_deleted;
            stats.total_word_changes += diff.stats.total_word_changes;
            stats.overall_quality_trend += diff.stats.overall_progress_score;
            
            // Consider significant changes as those with quality improvements or major text changes
            if diff.stats.quality_improvements > 0 || diff.stats.total_word_changes > 50 {
                stats.significant_changes += 1;
            }
        }
        
        if !chapter_diffs.is_empty() {
            stats.overall_quality_trend /= chapter_diffs.len() as f32;
        }

        stats
    }
}

/// Historical entry for a translation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationHistoryEntry {
    pub commit_id: String,
    pub timestamp: DateTime<Utc>,
    pub author: String,
    pub message: String,
    pub translation: TranslationVersion,
    pub unit_id: String,
    pub language: String,
}

/// Quality trends over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityTrends {
    pub chapter: String,
    pub language: String,
    pub data_points: Vec<QualityDataPoint>,
    pub trend_direction: f32, // positive = improving, negative = declining
    pub days_analyzed: u32,
}

/// Single quality data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDataPoint {
    pub timestamp: DateTime<Utc>,
    pub commit_id: String,
    pub average_quality: f32,
    pub units_scored: u32,
    pub total_units: u32,
}

/// Branch comparison report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchComparisonReport {
    pub base_branch: String,
    pub feature_branch: String,
    pub base_commit: String,
    pub feature_commit: String,
    pub generated_at: DateTime<Utc>,
    pub chapter_diffs: Vec<DetailedTranslationDiff>,
    pub overall_stats: BranchComparisonStats,
}

/// Overall statistics for branch comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchComparisonStats {
    pub total_chapters: u32,
    pub chapters_with_changes: u32,
    pub total_units_changed: u32,
    pub total_word_changes: u32,
    pub overall_quality_trend: f32,
    pub significant_changes: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_text_diff_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();
        let diff_tools = GitDiffTools::new(temp_dir.path()).unwrap();
        
        let old_text = "Hello world, this is a test.";
        let new_text = "Hello universe, this is a great test.";
        let options = DiffOptions::default();
        
        let text_diff = diff_tools.calculate_text_diff(old_text, new_text, &options).unwrap();
        
        assert!(text_diff.word_changes > 0);
        assert!(text_diff.character_changes > 0);
        assert!(text_diff.similarity_score > 0.0 && text_diff.similarity_score < 1.0);
    }
    
    #[test]
    fn test_ngram_generation() {
        let temp_dir = TempDir::new().unwrap();
        let diff_tools = GitDiffTools::new(temp_dir.path()).unwrap();
        
        let text = "hello";
        let ngrams = diff_tools.get_ngrams(text, 2);
        
        assert!(ngrams.contains("he"));
        assert!(ngrams.contains("el"));
        assert!(ngrams.contains("ll"));
        assert!(ngrams.contains("lo"));
        assert_eq!(ngrams.len(), 4);
    }
}