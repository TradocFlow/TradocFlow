#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::services::translation_memory_service::TranslationMemoryService;
    use std::sync::Arc;
    use tokio;
    use tempfile::TempDir;
    use uuid::Uuid;
    use std::collections::HashMap;

    async fn setup_test_services() -> (Arc<TranslationMemoryService>, Arc<ChunkLinkingService>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();
        
        let tm_service = Arc::new(
            TranslationMemoryService::new(project_path.clone())
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

    #[tokio::test]
    async fn test_selection_mode_validation() {
        let (_tm_service, linking_service, _temp_dir) = setup_test_services().await;
        
        let session_id = "test_validation_session".to_string();
        
        // Test individual selection mode
        linking_service
            .start_selection_session(session_id.clone(), SelectionMode::Individual)
            .await
            .unwrap();
        
        let selection = linking_service.get_selection(&session_id).await.unwrap().unwrap();
        assert_eq!(selection.selection_mode, SelectionMode::Individual);
        
        // Test range selection mode
        linking_service
            .start_selection_session(session_id.clone(), SelectionMode::Range)
            .await
            .unwrap();
        
        let selection = linking_service.get_selection(&session_id).await.unwrap().unwrap();
        assert_eq!(selection.selection_mode, SelectionMode::Range);
        
        // Test pattern selection mode
        linking_service
            .start_selection_session(session_id.clone(), SelectionMode::Pattern)
            .await
            .unwrap();
        
        let selection = linking_service.get_selection(&session_id).await.unwrap().unwrap();
        assert_eq!(selection.selection_mode, SelectionMode::Pattern);
    }

    #[tokio::test]
    async fn test_phrase_group_metadata_management() {
        let (_tm_service, linking_service, _temp_dir) = setup_test_services().await;
        
        let session_id = "test_metadata_session".to_string();
        
        // Create a phrase group
        linking_service
            .start_selection_session(session_id.clone(), SelectionMode::Individual)
            .await
            .unwrap();
        
        let chunk1 = Uuid::new_v4();
        let chunk2 = Uuid::new_v4();
        
        linking_service.add_chunk_to_selection(&session_id, chunk1).await.unwrap();
        linking_service.add_chunk_to_selection(&session_id, chunk2).await.unwrap();
        
        let result = linking_service
            .link_selected_chunks(&session_id, "Test phrase".to_string(), "en".to_string(), MergeOptions::default())
            .await
            .unwrap();
        
        // Update metadata
        let metadata = PhraseMetadata {
            creator_id: Some("user123".to_string()),
            description: Some("Test phrase for validation".to_string()),
            tags: vec!["test".to_string(), "validation".to_string()],
            confidence_score: 0.95,
            usage_count: 5,
        };
        
        linking_service
            .update_phrase_group_metadata(result.phrase_group_id, metadata.clone())
            .await
            .unwrap();
        
        // Verify metadata was updated
        let phrase_group = linking_service
            .get_phrase_group(result.phrase_group_id)
            .await
            .unwrap()
            .unwrap();
        
        assert_eq!(phrase_group.metadata.creator_id, metadata.creator_id);
        assert_eq!(phrase_group.metadata.description, metadata.description);
        assert_eq!(phrase_group.metadata.tags, metadata.tags);
        assert_eq!(phrase_group.metadata.confidence_score, metadata.confidence_score);
        assert_eq!(phrase_group.metadata.usage_count, metadata.usage_count);
    }

    #[tokio::test]
    async fn test_phrase_statistics() {
        let (_tm_service, linking_service, _temp_dir) = setup_test_services().await;
        
        // Create multiple phrase groups
        for i in 0..3 {
            let session_id = format!("test_stats_session_{}", i);
            
            linking_service
                .start_selection_session(session_id.clone(), SelectionMode::Individual)
                .await
                .unwrap();
            
            let chunk1 = Uuid::new_v4();
            let chunk2 = Uuid::new_v4();
            
            linking_service.add_chunk_to_selection(&session_id, chunk1).await.unwrap();
            linking_service.add_chunk_to_selection(&session_id, chunk2).await.unwrap();
            
            let language = if i == 0 { "en" } else if i == 1 { "es" } else { "fr" };
            
            let result = linking_service
                .link_selected_chunks(&session_id, format!("Test phrase {}", i), language.to_string(), MergeOptions::default())
                .await
                .unwrap();
            
            // Add some tags
            let metadata = PhraseMetadata {
                tags: vec![format!("tag{}", i), "common".to_string()],
                ..Default::default()
            };
            
            linking_service
                .update_phrase_group_metadata(result.phrase_group_id, metadata)
                .await
                .unwrap();
        }
        
        // Get statistics
        let stats = linking_service.get_phrase_statistics().await.unwrap();
        
        assert_eq!(stats.total_phrase_groups, 3);
        assert_eq!(stats.total_linked_chunks, 6); // 2 chunks per phrase group
        assert_eq!(stats.unique_languages, 3); // en, es, fr
        assert_eq!(stats.average_chunks_per_phrase, 2.0);
        
        // Check that "common" tag appears most frequently
        let common_tag_count = stats.most_used_tags
            .iter()
            .find(|(tag, _)| tag == "common")
            .map(|(_, count)| *count)
            .unwrap_or(0);
        
        assert_eq!(common_tag_count, 3);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let (_tm_service, linking_service, _temp_dir) = setup_test_services().await;
        
        // Test linking with insufficient chunks
        let session_id = "test_error_session".to_string();
        
        linking_service
            .start_selection_session(session_id.clone(), SelectionMode::Individual)
            .await
            .unwrap();
        
        let chunk1 = Uuid::new_v4();
        linking_service.add_chunk_to_selection(&session_id, chunk1).await.unwrap();
        
        let result = linking_service
            .link_selected_chunks(&session_id, "Test".to_string(), "en".to_string(), MergeOptions::default())
            .await
            .unwrap();
        
        assert!(!result.success);
        assert!(result.message.contains("At least 2 chunks"));
        
        // Test operations on non-existent session
        let result = linking_service.add_chunk_to_selection("non_existent", chunk1).await;
        assert!(result.is_err());
        
        // Test operations on non-existent phrase group
        let result = linking_service.unlink_phrase_group(Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_sessions() {
        let (_tm_service, linking_service, _temp_dir) = setup_test_services().await;
        
        // Create multiple concurrent sessions
        let session1 = "concurrent_session_1".to_string();
        let session2 = "concurrent_session_2".to_string();
        
        linking_service
            .start_selection_session(session1.clone(), SelectionMode::Individual)
            .await
            .unwrap();
        
        linking_service
            .start_selection_session(session2.clone(), SelectionMode::Range)
            .await
            .unwrap();
        
        // Add different chunks to each session
        let chunk1 = Uuid::new_v4();
        let chunk2 = Uuid::new_v4();
        let chunk3 = Uuid::new_v4();
        let chunk4 = Uuid::new_v4();
        
        linking_service.add_chunk_to_selection(&session1, chunk1).await.unwrap();
        linking_service.add_chunk_to_selection(&session1, chunk2).await.unwrap();
        
        linking_service.add_chunk_to_selection(&session2, chunk3).await.unwrap();
        linking_service.add_chunk_to_selection(&session2, chunk4).await.unwrap();
        
        // Verify sessions are independent
        let selection1 = linking_service.get_selection(&session1).await.unwrap().unwrap();
        let selection2 = linking_service.get_selection(&session2).await.unwrap().unwrap();
        
        assert_eq!(selection1.selected_chunks.len(), 2);
        assert_eq!(selection2.selected_chunks.len(), 2);
        assert_eq!(selection1.selection_mode, SelectionMode::Individual);
        assert_eq!(selection2.selection_mode, SelectionMode::Range);
        
        assert!(selection1.selected_chunks.contains(&chunk1));
        assert!(selection1.selected_chunks.contains(&chunk2));
        assert!(!selection1.selected_chunks.contains(&chunk3));
        assert!(!selection1.selected_chunks.contains(&chunk4));
        
        assert!(selection2.selected_chunks.contains(&chunk3));
        assert!(selection2.selected_chunks.contains(&chunk4));
        assert!(!selection2.selected_chunks.contains(&chunk1));
        assert!(!selection2.selected_chunks.contains(&chunk2));
    }
}