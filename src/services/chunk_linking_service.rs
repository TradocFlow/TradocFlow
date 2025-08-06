use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

use crate::services::translation_memory_adapter::TranslationMemoryAdapter;

/// Service for managing chunk linking and phrase groups
pub struct ChunkLinkingService {
    translation_memory: Arc<TranslationMemoryAdapter>,
    linked_phrases: Arc<RwLock<HashMap<Uuid, LinkedPhraseGroup>>>,
    chunk_selections: Arc<RwLock<HashMap<String, ChunkSelection>>>, // session_id -> selection
}

/// Represents a group of linked chunks that form a phrase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedPhraseGroup {
    pub id: Uuid,
    pub chunk_ids: Vec<Uuid>,
    pub phrase_text: String,
    pub language: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: PhraseMetadata,
}

/// Metadata for linked phrase groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseMetadata {
    pub creator_id: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub confidence_score: f32,
    pub usage_count: u32,
}

impl Default for PhraseMetadata {
    fn default() -> Self {
        Self {
            creator_id: None,
            description: None,
            tags: Vec::new(),
            confidence_score: 1.0,
            usage_count: 0,
        }
    }
}

/// Represents a user's current chunk selection for linking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkSelection {
    pub session_id: String,
    pub selected_chunks: Vec<Uuid>,
    pub selection_mode: SelectionMode,
    pub created_at: DateTime<Utc>,
}

/// Different modes for chunk selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SelectionMode {
    /// Select individual chunks for linking
    Individual,
    /// Select a range of chunks
    Range,
    /// Select chunks based on text pattern
    Pattern,
}

/// Result of a chunk linking operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkingResult {
    pub phrase_group_id: Uuid,
    pub linked_chunks: Vec<Uuid>,
    pub merged_text: String,
    pub success: bool,
    pub message: String,
}

/// Options for chunk merging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeOptions {
    pub preserve_formatting: bool,
    pub add_spacing: bool,
    pub merge_strategy: MergeStrategy,
    pub update_translation_memory: bool,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            preserve_formatting: true,
            add_spacing: true,
            merge_strategy: MergeStrategy::Sequential,
            update_translation_memory: true,
        }
    }
}

/// Strategy for merging chunks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MergeStrategy {
    /// Merge chunks in their original order
    Sequential,
    /// Merge chunks based on position
    Positional,
    /// Custom merge order
    Custom(Vec<Uuid>),
}

impl ChunkLinkingService {
    /// Create a new chunk linking service
    pub async fn new(translation_memory: Arc<TranslationMemoryAdapter>) -> Result<Self> {
        Ok(Self {
            translation_memory,
            linked_phrases: Arc::new(RwLock::new(HashMap::new())),
            chunk_selections: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start a new chunk selection session
    pub async fn start_selection_session(&self, session_id: String, mode: SelectionMode) -> Result<()> {
        let selection = ChunkSelection {
            session_id: session_id.clone(),
            selected_chunks: Vec::new(),
            selection_mode: mode,
            created_at: Utc::now(),
        };

        let mut selections = self.chunk_selections.write().await;
        selections.insert(session_id, selection);
        Ok(())
    }

    /// Add a chunk to the current selection
    pub async fn add_chunk_to_selection(&self, session_id: &str, chunk_id: Uuid) -> Result<()> {
        let mut selections = self.chunk_selections.write().await;
        
        if let Some(selection) = selections.get_mut(session_id) {
            if !selection.selected_chunks.contains(&chunk_id) {
                selection.selected_chunks.push(chunk_id);
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Selection session not found: {}", session_id))
        }
    }

    /// Remove a chunk from the current selection
    pub async fn remove_chunk_from_selection(&self, session_id: &str, chunk_id: &Uuid) -> Result<()> {
        let mut selections = self.chunk_selections.write().await;
        
        if let Some(selection) = selections.get_mut(session_id) {
            selection.selected_chunks.retain(|id| id != chunk_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Selection session not found: {}", session_id))
        }
    }

    /// Get the current selection for a session
    pub async fn get_selection(&self, session_id: &str) -> Result<Option<ChunkSelection>> {
        let selections = self.chunk_selections.read().await;
        Ok(selections.get(session_id).cloned())
    }

    /// Clear the selection for a session
    pub async fn clear_selection(&self, session_id: &str) -> Result<()> {
        let mut selections = self.chunk_selections.write().await;
        
        if let Some(selection) = selections.get_mut(session_id) {
            selection.selected_chunks.clear();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Selection session not found: {}", session_id))
        }
    }

    /// Link selected chunks into a phrase group
    pub async fn link_selected_chunks(
        &self,
        session_id: &str,
        phrase_text: String,
        language: String,
        options: MergeOptions,
    ) -> Result<LinkingResult> {
        let selection = {
            let selections = self.chunk_selections.read().await;
            selections.get(session_id).cloned()
                .ok_or_else(|| anyhow::anyhow!("Selection session not found: {}", session_id))?
        };

        if selection.selected_chunks.len() < 2 {
            return Ok(LinkingResult {
                phrase_group_id: Uuid::new_v4(),
                linked_chunks: Vec::new(),
                merged_text: String::new(),
                success: false,
                message: "At least 2 chunks must be selected for linking".to_string(),
            });
        }

        // Validate that chunks can be linked
        self.validate_chunks_for_linking(&selection.selected_chunks).await?;

        // Create the linked phrase group
        let phrase_group = LinkedPhraseGroup {
            id: Uuid::new_v4(),
            chunk_ids: selection.selected_chunks.clone(),
            phrase_text: phrase_text.clone(),
            language,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: PhraseMetadata::default(),
        };

        // Store the phrase group
        {
            let mut phrases = self.linked_phrases.write().await;
            phrases.insert(phrase_group.id, phrase_group.clone());
        }

        // Update chunk metadata to link them
        self.translation_memory
            .update_chunk_linking(selection.selected_chunks.clone(), crate::services::translation_memory_adapter::ChunkLinkType::LinkedPhrase)
            .await?;

        // Update translation memory if requested
        if options.update_translation_memory {
            self.update_translation_memory_for_phrase(&phrase_group).await?;
        }

        // Clear the selection
        self.clear_selection(session_id).await?;

        Ok(LinkingResult {
            phrase_group_id: phrase_group.id,
            linked_chunks: phrase_group.chunk_ids,
            merged_text: phrase_text,
            success: true,
            message: "Chunks successfully linked into phrase group".to_string(),
        })
    }

    /// Unlink a phrase group
    pub async fn unlink_phrase_group(&self, phrase_group_id: Uuid) -> Result<()> {
        let phrase_group = {
            let mut phrases = self.linked_phrases.write().await;
            phrases.remove(&phrase_group_id)
                .ok_or_else(|| anyhow::anyhow!("Phrase group not found: {}", phrase_group_id))?
        };

        // Update chunk metadata to unlink them
        self.translation_memory
            .update_chunk_linking(phrase_group.chunk_ids, crate::services::translation_memory_adapter::ChunkLinkType::Unlinked)
            .await?;

        Ok(())
    }

    /// Get all linked phrase groups
    pub async fn get_all_phrase_groups(&self) -> Result<Vec<LinkedPhraseGroup>> {
        let phrases = self.linked_phrases.read().await;
        Ok(phrases.values().cloned().collect())
    }

    /// Get a specific phrase group
    pub async fn get_phrase_group(&self, phrase_group_id: Uuid) -> Result<Option<LinkedPhraseGroup>> {
        let phrases = self.linked_phrases.read().await;
        Ok(phrases.get(&phrase_group_id).cloned())
    }

    /// Update phrase group metadata
    pub async fn update_phrase_group_metadata(
        &self,
        phrase_group_id: Uuid,
        metadata: PhraseMetadata,
    ) -> Result<()> {
        let mut phrases = self.linked_phrases.write().await;
        
        if let Some(phrase_group) = phrases.get_mut(&phrase_group_id) {
            phrase_group.metadata = metadata;
            phrase_group.updated_at = Utc::now();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Phrase group not found: {}", phrase_group_id))
        }
    }

    /// Merge chunks with specified options
    pub async fn merge_chunks(
        &self,
        chunk_ids: Vec<Uuid>,
        chunk_content: &HashMap<Uuid, String>,
        options: MergeOptions,
    ) -> Result<String> {
        if chunk_ids.is_empty() {
            return Ok(String::new());
        }

        let ordered_chunks = match options.merge_strategy {
            MergeStrategy::Sequential => chunk_ids,
            MergeStrategy::Positional => {
                // Sort by original position (would need chunk metadata)
                chunk_ids // Simplified for now
            }
            MergeStrategy::Custom(order) => order,
        };

        let mut merged_text = String::new();
        
        for (i, chunk_id) in ordered_chunks.iter().enumerate() {
            if let Some(content) = chunk_content.get(chunk_id) {
                if i > 0 && options.add_spacing {
                    merged_text.push(' ');
                }
                merged_text.push_str(content);
            }
        }

        Ok(merged_text)
    }

    /// Get chunks that are linked to a specific chunk
    pub async fn get_linked_chunks(&self, chunk_id: Uuid) -> Result<Vec<Uuid>> {
        let phrases = self.linked_phrases.read().await;
        
        for phrase_group in phrases.values() {
            if phrase_group.chunk_ids.contains(&chunk_id) {
                return Ok(phrase_group.chunk_ids.clone());
            }
        }

        Ok(Vec::new())
    }

    /// Search for phrase groups by text
    pub async fn search_phrase_groups(&self, query: &str) -> Result<Vec<LinkedPhraseGroup>> {
        let phrases = self.linked_phrases.read().await;
        let query_lower = query.to_lowercase();
        
        let matching_phrases = phrases
            .values()
            .filter(|phrase| {
                phrase.phrase_text.to_lowercase().contains(&query_lower) ||
                phrase.metadata.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect();

        Ok(matching_phrases)
    }

    /// Get statistics about linked phrases
    pub async fn get_phrase_statistics(&self) -> Result<PhraseStatistics> {
        let phrases = self.linked_phrases.read().await;
        
        let total_phrases = phrases.len();
        let total_chunks = phrases.values().map(|p| p.chunk_ids.len()).sum();
        let languages: HashSet<String> = phrases.values().map(|p| p.language.clone()).collect();
        let avg_chunks_per_phrase = if total_phrases > 0 {
            total_chunks as f32 / total_phrases as f32
        } else {
            0.0
        };

        Ok(PhraseStatistics {
            total_phrase_groups: total_phrases,
            total_linked_chunks: total_chunks,
            unique_languages: languages.len(),
            average_chunks_per_phrase: avg_chunks_per_phrase,
            most_used_tags: self.get_most_used_tags(&phrases).await,
        })
    }

    /// Validate that chunks can be linked together
    async fn validate_chunks_for_linking(&self, chunk_ids: &[Uuid]) -> Result<()> {
        if chunk_ids.len() < 2 {
            return Err(anyhow::anyhow!("At least 2 chunks required for linking"));
        }

        // Check for duplicate IDs
        let unique_ids: HashSet<_> = chunk_ids.iter().collect();
        if unique_ids.len() != chunk_ids.len() {
            return Err(anyhow::anyhow!("Duplicate chunk IDs in selection"));
        }

        // Additional validation could be added here
        // - Check if chunks are from the same chapter
        // - Check if chunks are compatible types
        // - Check if chunks are not already linked

        Ok(())
    }

    /// Update translation memory with phrase group information
    async fn update_translation_memory_for_phrase(&self, _phrase_group: &LinkedPhraseGroup) -> Result<()> {
        // This would create translation units for the linked phrase
        // Implementation depends on how translation memory handles phrase groups
        
        // For now, we'll just mark the chunks as linked in the translation memory
        // The actual translation memory update would happen in the translation memory service
        
        Ok(())
    }

    /// Get the most frequently used tags
    async fn get_most_used_tags(&self, phrases: &HashMap<Uuid, LinkedPhraseGroup>) -> Vec<(String, u32)> {
        let mut tag_counts: HashMap<String, u32> = HashMap::new();
        
        for phrase in phrases.values() {
            for tag in &phrase.metadata.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        let mut sorted_tags: Vec<_> = tag_counts.into_iter().collect();
        sorted_tags.sort_by(|a, b| b.1.cmp(&a.1));
        sorted_tags.truncate(10); // Top 10 tags
        
        sorted_tags
    }
}

/// Statistics about phrase groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseStatistics {
    pub total_phrase_groups: usize,
    pub total_linked_chunks: usize,
    pub unique_languages: usize,
    pub average_chunks_per_phrase: f32,
    pub most_used_tags: Vec<(String, u32)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio;

    #[tokio::test]
    async fn test_chunk_selection_session() {
        let tm_service = Arc::new(
            TranslationMemoryAdapter::new(std::path::PathBuf::from("/tmp/test"))
                .await
                .unwrap()
        );
        let service = ChunkLinkingService::new(tm_service).await.unwrap();
        
        let session_id = "test_session".to_string();
        
        // Start selection session
        service.start_selection_session(session_id.clone(), SelectionMode::Individual)
            .await
            .unwrap();
        
        // Add chunks to selection
        let chunk1 = Uuid::new_v4();
        let chunk2 = Uuid::new_v4();
        
        service.add_chunk_to_selection(&session_id, chunk1).await.unwrap();
        service.add_chunk_to_selection(&session_id, chunk2).await.unwrap();
        
        // Get selection
        let selection = service.get_selection(&session_id).await.unwrap().unwrap();
        assert_eq!(selection.selected_chunks.len(), 2);
        assert!(selection.selected_chunks.contains(&chunk1));
        assert!(selection.selected_chunks.contains(&chunk2));
        
        // Clear selection
        service.clear_selection(&session_id).await.unwrap();
        let selection = service.get_selection(&session_id).await.unwrap().unwrap();
        assert_eq!(selection.selected_chunks.len(), 0);
    }

    #[tokio::test]
    async fn test_chunk_merging() {
        let tm_service = Arc::new(
            TranslationMemoryAdapter::new(std::path::PathBuf::from("/tmp/test"))
                .await
                .unwrap()
        );
        let service = ChunkLinkingService::new(tm_service).await.unwrap();
        
        let chunk1 = Uuid::new_v4();
        let chunk2 = Uuid::new_v4();
        let chunk3 = Uuid::new_v4();
        
        let mut chunk_content = HashMap::new();
        chunk_content.insert(chunk1, "First chunk".to_string());
        chunk_content.insert(chunk2, "second chunk".to_string());
        chunk_content.insert(chunk3, "third chunk".to_string());
        
        let chunk_ids = vec![chunk1, chunk2, chunk3];
        let options = MergeOptions::default();
        
        let merged = service.merge_chunks(chunk_ids, &chunk_content, options)
            .await
            .unwrap();
        
        assert_eq!(merged, "First chunk second chunk third chunk");
    }

    #[tokio::test]
    async fn test_phrase_group_management() {
        let tm_service = Arc::new(
            TranslationMemoryAdapter::new(std::path::PathBuf::from("/tmp/test"))
                .await
                .unwrap()
        );
        let service = ChunkLinkingService::new(tm_service).await.unwrap();
        
        let session_id = "test_session".to_string();
        service.start_selection_session(session_id.clone(), SelectionMode::Individual)
            .await
            .unwrap();
        
        let chunk1 = Uuid::new_v4();
        let chunk2 = Uuid::new_v4();
        
        service.add_chunk_to_selection(&session_id, chunk1).await.unwrap();
        service.add_chunk_to_selection(&session_id, chunk2).await.unwrap();
        
        // This test would need a mock translation memory service to work properly
        // For now, we'll just test the basic structure
        
        let stats = service.get_phrase_statistics().await.unwrap();
        assert_eq!(stats.total_phrase_groups, 0);
    }
}

// Integration tests are included inline for now

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::services::translation_memory_adapter::TranslationMemoryAdapter;
    use std::sync::Arc;
    use tokio;
    use tempfile::TempDir;
    use uuid::Uuid;
    use std::collections::HashMap;

    async fn setup_test_services() -> (Arc<TranslationMemoryAdapter>, Arc<ChunkLinkingService>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();
        
        let tm_service = Arc::new(
            TranslationMemoryAdapter::new(project_path.clone())
                .await
                .unwrap()
        );
        
        let linking_service = Arc::new(
            ChunkLinkingService::new(tm_service.clone())
                .await
                .unwrap()
        );
        
        (tm_service, linking_service, temp_dir)
    }

    #[tokio::test]
    async fn test_complete_chunk_linking_workflow() {
        let (_tm_service, linking_service, _temp_dir) = setup_test_services().await;
        
        let session_id = "test_workflow_session".to_string();
        
        // Step 1: Start a linking session
        linking_service
            .start_selection_session(session_id.clone(), SelectionMode::Individual)
            .await
            .unwrap();
        
        // Step 2: Add chunks to selection
        let chunk1 = Uuid::new_v4();
        let chunk2 = Uuid::new_v4();
        let chunk3 = Uuid::new_v4();
        
        linking_service.add_chunk_to_selection(&session_id, chunk1).await.unwrap();
        linking_service.add_chunk_to_selection(&session_id, chunk2).await.unwrap();
        linking_service.add_chunk_to_selection(&session_id, chunk3).await.unwrap();
        
        // Step 3: Verify selection
        let selection = linking_service.get_selection(&session_id).await.unwrap().unwrap();
        assert_eq!(selection.selected_chunks.len(), 3);
        assert!(selection.selected_chunks.contains(&chunk1));
        assert!(selection.selected_chunks.contains(&chunk2));
        assert!(selection.selected_chunks.contains(&chunk3));
        
        // Step 4: Link chunks into phrase group
        let phrase_text = "This is a linked phrase".to_string();
        let language = "en".to_string();
        let options = MergeOptions::default();
        
        let result = linking_service
            .link_selected_chunks(&session_id, phrase_text.clone(), language.clone(), options)
            .await
            .unwrap();
        
        assert!(result.success);
        assert_eq!(result.linked_chunks.len(), 3);
        assert_eq!(result.merged_text, phrase_text);
        
        // Step 5: Verify phrase group was created
        let phrase_group = linking_service
            .get_phrase_group(result.phrase_group_id)
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(phrase_group.phrase_text, phrase_text);
        assert_eq!(phrase_group.language, language);
        assert_eq!(phrase_group.chunk_ids.len(), 3);
        
        // Step 6: Verify selection was cleared
        let selection = linking_service.get_selection(&session_id).await.unwrap().unwrap();
        assert_eq!(selection.selected_chunks.len(), 0);
        
        // Step 7: Test phrase group search
        let search_results = linking_service
            .search_phrase_groups("linked phrase")
            .await
            .unwrap();
        
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].id, phrase_group.id);
        
        // Step 8: Test getting linked chunks
        let linked_chunks = linking_service.get_linked_chunks(chunk1).await.unwrap();
        assert_eq!(linked_chunks.len(), 3);
        assert!(linked_chunks.contains(&chunk1));
        assert!(linked_chunks.contains(&chunk2));
        assert!(linked_chunks.contains(&chunk3));
        
        // Step 9: Test unlinking
        linking_service.unlink_phrase_group(phrase_group.id).await.unwrap();
        
        // Step 10: Verify phrase group was removed
        let phrase_group = linking_service
            .get_phrase_group(result.phrase_group_id)
            .await
            .unwrap();
        assert!(phrase_group.is_none());
    }

    #[tokio::test]
    async fn test_chunk_merging_strategies() {
        let (_tm_service, linking_service, _temp_dir) = setup_test_services().await;
        
        let chunk1 = Uuid::new_v4();
        let chunk2 = Uuid::new_v4();
        let chunk3 = Uuid::new_v4();
        
        let mut chunk_content = HashMap::new();
        chunk_content.insert(chunk1, "First".to_string());
        chunk_content.insert(chunk2, "second".to_string());
        chunk_content.insert(chunk3, "third".to_string());
        
        let chunk_ids = vec![chunk1, chunk2, chunk3];
        
        // Test sequential merge
        let sequential_options = MergeOptions {
            merge_strategy: MergeStrategy::Sequential,
            add_spacing: true,
            preserve_formatting: true,
            update_translation_memory: false,
        };
        
        let merged = linking_service
            .merge_chunks(chunk_ids.clone(), &chunk_content, sequential_options)
            .await
            .unwrap();
        
        assert_eq!(merged, "First second third");
        
        // Test merge without spacing
        let no_spacing_options = MergeOptions {
            merge_strategy: MergeStrategy::Sequential,
            add_spacing: false,
            preserve_formatting: true,
            update_translation_memory: false,
        };
        
        let merged_no_spacing = linking_service
            .merge_chunks(chunk_ids.clone(), &chunk_content, no_spacing_options)
            .await
            .unwrap();
        
        assert_eq!(merged_no_spacing, "Firstsecondthird");
        
        // Test custom order merge
        let custom_order = vec![chunk3, chunk1, chunk2];
        let custom_options = MergeOptions {
            merge_strategy: MergeStrategy::Custom(custom_order),
            add_spacing: true,
            preserve_formatting: true,
            update_translation_memory: false,
        };
        
        let merged_custom = linking_service
            .merge_chunks(chunk_ids, &chunk_content, custom_options)
            .await
            .unwrap();
        
        assert_eq!(merged_custom, "third First second");
    }
}